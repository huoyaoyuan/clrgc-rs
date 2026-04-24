use bitflags::bitflags;

/// The common part of an object on GC heap. GC only defines the necessary fields for calculating object size.
/// 
/// GC calculates the object size simply by `BaseSize + (ComponentSize * ComponentCount)`.
/// For non variable-sized objects, the object payload starts at the space of [`Object::component_count`], which will be multiplied by 0 anyway.
/// 
/// The layout of an object on heap is as following:
/// ```text
///   1 pointer      1 pointer        pointer aligned
/// +--------------+----------------+----------------+----------------+----------------+
/// | ObjectHeader | MethodTablePtr | ComponentCount/Payload...       | Padding...     |
/// +--------------+----------------+----------------+----------------+----------------+
///                ^
///                |
///                ObjectRef points here
/// ```
/// 
/// Any manipulation code must be aware of the header at negative offset.
/// Only the low 32 bits of the header are used.
#[repr(C)]
pub struct Object {
    pub method_table: *const MethodTable,
    pub component_count: u32,
}

/// [`MethodTable`] represents the type of any object on the GC heap.
/// GC only uses it to calculate object size and determine several flags.
#[repr(C)]
pub struct MethodTable {
    /// The component size for variable-sized objects, contains required padding for alignment of elements.
    /// 
    /// When [`MethodTable::flags_high`] doesn't contain [`Object::HAS_COMPONENT_SIZE`], the space may be used for other flags.
    pub component_size: u16,

    pub flags_high: u16,

    /// The base size of the type.
    /// - For fix-sized objects, the base size has been aligned to pointer size.
    /// - For variable-sized objects, the base size can contain unaligned payloads, and the object size should be aligned after adding the elements part.
    pub base_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GCDescSeries {
    /// Size of a series of object references in bytes, substracted by the object size.
    /// Always negative.
    pub size: isize,
    /// Offset of a series of object references in bytes, counting from the object pointer.
    pub offset: usize,
}

#[cfg(target_pointer_width = "64")]
#[repr(C)]
#[derive(Clone, Copy)]
struct ValSeriesItem {
    /// The number of object references in pointers
    pub pointers: u32,
    /// The size of non-object reference payload in bytes
    pub skip: u32,
}

#[cfg(target_pointer_width = "32")]
#[repr(C)]
#[derive(Clone, Copy)]
struct ValSeriesItem {
    /// The number of object references in pointers
    pub pointers: u16,
    /// The size of non-object reference payload in bytes
    pub skip: u16,
}

bitflags! {
    #[repr(transparent)]
    struct ObjectHeader : u32 {
        /// The [`ObjectHeader::BIT_SBLK_FINALIZER_RUN`] bit is used by CLR for skipping finalization of objects.
        /// Note that the finalization thread clears the bit when invoking the finalizer, so GC should not use it for its own tracking.
        const BIT_SBLK_FINALIZER_RUN = 0x40000000;
    }
}

pub type ObjectRef = *mut Object;

pub fn align_to_ptr(size: usize) -> usize {
    let mask = size_of::<usize>() - 1;
    (size + mask) & !mask
}

/// The free method table used for representing gaps on the heap.
/// It's a variable sized MT with component size 1 and can represent any size of free space from the minimum object size.
/// 
/// The value is retrieved from CLR via [`crate::gcinterface::GCToCLR::get_free_methodtable`]
pub static mut FREE_MT: *const MethodTable = std::ptr::null();

impl Object {
    pub const HAS_COMPONENT_SIZE: u16 = 0x8000;
    pub const HAS_GC_POINTERS: u16 = 0x0100;
    pub const HAS_FINALIZER: u16 = 0x0010;
    pub const BASE_SIZE: usize = 3 * size_of::<usize>();

    #[inline]
    pub fn has_component_size(&self) -> bool {
        let mt = unsafe { &*self.method_table };
        mt.flags_high & Self::HAS_COMPONENT_SIZE != 0
    }

    #[inline]
    pub fn is_finalizable(&self) -> bool {
        let mt = unsafe { &*self.method_table };
        mt.flags_high & Self::HAS_FINALIZER != 0
    }

    #[inline]
    pub fn get_finalizer_run(&mut self) -> bool {
        let header = (self as ObjectRef as *mut usize).wrapping_sub(1) as *const ObjectHeader;
        unsafe { (*header).contains(ObjectHeader::BIT_SBLK_FINALIZER_RUN) }
    }

    #[inline]
    pub fn set_finalizer_run(&mut self, value: bool) {
        let header = (self as ObjectRef as *mut usize).wrapping_sub(1) as *mut ObjectHeader;
        unsafe { (*header).set(ObjectHeader::BIT_SBLK_FINALIZER_RUN, value); }
    }

    #[inline]
    pub fn needs_finalization(&mut self) -> bool {
        self.is_finalizable() && !self.get_finalizer_run()
    }

    #[inline]
    pub fn total_size(&self) -> u32 {
        let mt = unsafe { &*self.method_table };
        mt.base_size + if self.has_component_size() { mt.component_size as u32 * self.component_count } else { 0 }
    }

    #[inline]
    pub fn total_size_aligned(&self) -> usize {
        align_to_ptr(self.total_size() as usize)
    }

    /// Walk through the reference fields in the object, including any nested structs.
    /// There can't be interior pointers and all the reference fields should point to the start of an object.
    pub fn for_each_obj_ref<F: FnMut(&mut ObjectRef)>(&mut self, mut f: F) {
        unsafe {
            if (*self.method_table).flags_high & Self::HAS_GC_POINTERS == 0 {
                return;
            }

            // Decode the GCDesc about the object layout.
            // GCDesc is encoded at negative offset from the method table object:
            // 
            //       [2]      [1]      1 pointer
            // ----------------------+--------------+------------------------
            // ... | Series | Series | Series count | MethodTable content ...
            // ----------------------+--------------+------------------------
            //                                        ^
            //                                        | MethodTable pointer
            let base_ptr = (self.method_table as *const isize).sub(1);
            let series_count = *base_ptr;

            if series_count >= 0 {
                // Positive series count is used for regular cases, which uses the GCDescSeries encoding.
                // A GCDescSeries represents a continuous range of object references in the object.
                // - For fix-sized objects, there layout of every object is the same. There is usually only 1 serie since CLR optimizes object layout.
                // - For reference type arrays, there is a single serie of all the elements, but its length varies for each object.
                //
                // GCDescSeries handles array by substracting object size from series size, so that arrays of different length can share the same encoded value.
                // The serie length will be -BaseSize and adding by object size results in ComponentSize * ComponentCount.
                let obj_size = self.total_size_aligned() as isize;
                for s in 1..series_count + 1 {
                    let gc_desc_base = base_ptr as *const GCDescSeries;
                    let series = *gc_desc_base.sub(s as usize);
                    let field_count = (obj_size + series.size) as usize / size_of::<usize>();
                    let series_ptr = (&raw mut self.method_table as *mut ObjectRef).byte_add(series.offset);

                    for i in 0..field_count {
                        f(&mut *series_ptr.add(i));
                    }
                }
            } else {
                // Negative series count is used for encoding value type arrays, which can contain repeating series of references.
                let component_size = (*self.method_table).component_size as usize;
                let offset = *base_ptr.sub(1);
                let val_series_base = base_ptr.sub(2) as *const ValSeriesItem;
                let elements_base = (&raw mut self.method_table as *mut ObjectRef).byte_offset(offset);

                // ValSeriesItem encodes the references within one element so we need to iterate through the elements.
                // Offsets are represented by the relative location from last serie.
                for e in 0..self.component_count as usize {
                    let mut element_ptr = elements_base.byte_add(e * component_size);
                    for s in 0..(-series_count as usize) {
                        let item = *val_series_base.sub(s);
                        for i in 0..item.pointers as usize {
                            f(&mut *element_ptr.add(i));
                        }
                        element_ptr = element_ptr.add(item.pointers as usize).byte_add(item.skip as usize);
                    }
                }
            }
        }
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum HandleType {
    #[default]
    /// A weak GC handle.
    /// The handle is cleared when the object is unreachable, not encountering finalizable objects.
    /// Handles to finalizable object (and its fields) are cleared before finalizer execution.
    Short = 0,

    /// A recursion-tracking weak GC handle.
    /// The handle is cleared when the object is unreachable encountering finalizable objects, and really eligible for clearing.
    /// Handles to finalizable object (and its fields) are cleared after finalizer execution when not re-registered to finalization.
    ShortRecurrsion = 1,

    /// A strong GC handle, keeps the object alive.
    Strong = 2,

    /// A pinned GC handle, keeps the object alive and address unchanged.
    Pinned = 3,
    
    /// A dependent GC handle, acts like an additional field of primary object.
    /// - When primary object is reachable, keeps secondary object alive and reachable.
    /// - When primary object is eligible for finalization, allows secondary object for finalization like a recursion-tracking handle, but keeps uncleared.
    /// - When primary object is null or dead, the secondary reference will be cleared no matter the lifetime of the object.
    Dependent = 6,
}

/// The GC handle struct in table.
/// CLR code depends on the fact that derefencing [`ObjectHandle`] gets the object reference,
/// so [`GcHandle::object`] must be the first field, and the [`GcHandle`] structure must not be relocated during its lifetime.
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct GcHandle {
    pub object: ObjectRef,
    pub extra_or_secondary: usize,
    pub handle_type: HandleType,
}

pub type ObjectHandle = *mut GcHandle;
