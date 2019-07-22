use core::{iter, mem, ops, ptr::NonNull, slice};
use std::rc::Rc;

use ffi::csmModel;

use crate::{error::CubismResult, mem::AlignedMemory, moc::Moc, ConstantFlags, DynamicFlags};

/// This represents a model.
///
/// A model shares its underlying [Moc](./struct.Moc.html) with other models
/// that have been cloned from this one. Because of this it is preferred to
/// clone models, instead of creating new ones from the same data.
///
/// Slices returned by functions have to be indexed by the drawable, parameter
/// or part index for the individual value. If the functions takes an index
/// argument, then the function index replaces this behaviour and the returned
/// slice are values that all belong to the drawable.
#[derive(Debug)]
pub struct Model {
    mem: AlignedMemory<csmModel>,
    moc: Rc<Moc>,
    param_val: NonNull<[f32]>,
    part_opacities: NonNull<[f32]>,
    drawable_count: usize,
}

impl Model {
    /// Creates a model instance from bytes.
    #[inline]
    pub fn from_bytes<R: AsRef<[u8]>>(data: R) -> CubismResult<Self> {
        unsafe { Moc::new(data.as_ref()).map(|(moc, mem)| Self::new_impl(Rc::new(moc), mem)) }
    }

    pub fn parameter(&self, name: &str) -> Option<Parameter> {
        self.parameter_ids()
            .iter()
            .enumerate()
            .find_map(|(idx, id)| {
                if *id == name {
                    Some(self.parameter_at(idx))
                } else {
                    None
                }
            })
    }

    pub fn parameter_mut(&mut self, name: &str) -> Option<ParameterMut> {
        if let Some(idx) = self.parameter_ids().iter().position(|id| *id == name) {
            Some(self.parameter_at_mut(idx))
        } else {
            None
        }
    }

    pub fn parameter_at(&self, idx: usize) -> Parameter {
        // Do manual bounds checking since all slices have the same length
        assert!(idx < self.parameter_count());
        unsafe {
            Parameter {
                id: &self.param_ids.get_unchecked(idx),
                value: *self.parameter_values().get_unchecked(idx),
                min_value: *self.parameter_min().get_unchecked(idx),
                max_value: *self.parameter_max().get_unchecked(idx),
                default_value: *self.parameter_default().get_unchecked(idx),
            }
        }
    }

    pub fn parameter_at_mut(&mut self, idx: usize) -> ParameterMut {
        // Do manual bounds checking since all slices have the same length
        assert!(idx < self.parameter_count());
        unsafe {
            let min_value = *self.parameter_min().get_unchecked(idx);
            let max_value = *self.parameter_max().get_unchecked(idx);
            let default_value = *self.parameter_default().get_unchecked(idx);
            ParameterMut {
                id: &self.moc.param_ids.get_unchecked(idx),
                value: self.parameter_values_mut().get_unchecked_mut(idx),
                min_value,
                max_value,
                default_value,
            }
        }
    }

    pub fn part(&self, name: &str) -> Option<Part> {
        self.part_ids().iter().enumerate().find_map(|(idx, id)| {
            if *id == name {
                Some(self.part_at(idx))
            } else {
                None
            }
        })
    }

    pub fn part_mut(&mut self, name: &str) -> Option<PartMut> {
        if let Some(idx) = self.part_ids().iter().position(|id| *id == name) {
            Some(self.part_at_mut(idx))
        } else {
            None
        }
    }

    #[inline]
    pub fn part_at(&self, idx: usize) -> Part {
        Part {
            id: &self.moc.part_ids[idx],
            opacity: self.part_opacities()[idx],
        }
    }

    #[inline]
    pub fn part_at_mut(&mut self, idx: usize) -> PartMut {
        PartMut {
            id: &self.moc.part_ids[idx],
            opacity: &mut self.part_opacities_mut()[idx],
        }
    }

    /// Returns the parameter values.
    #[inline]
    pub fn parameter_values(&self) -> &[f32] {
        unsafe { self.param_val.as_ref() }
    }

    /// Returns a mutable slice of the parameter values.
    #[inline]
    pub fn parameter_values_mut(&mut self) -> &mut [f32] {
        unsafe { self.param_val.as_mut() }
    }

    /// Sets the parameter value at index `idx` to `val`.
    #[inline]
    pub fn set_parameter_value(&mut self, idx: usize, val: f32) {
        self.parameter_values_mut()[idx] = val;
    }

    /// Returns the part opacities.
    #[inline]
    pub fn part_opacities(&self) -> &[f32] {
        unsafe { self.part_opacities.as_ref() }
    }

    /// Returns a mutable slice of the part opacities.
    #[inline]
    pub fn part_opacities_mut(&mut self) -> &mut [f32] {
        unsafe { self.part_opacities.as_mut() }
    }

    /// Sets the part opacity at index `idx` to `val`.
    #[inline]
    pub fn set_part_opacity(&mut self, idx: usize, val: f32) {
        self.part_opacities_mut()[idx] = val;
    }

    /// Updates this model and finalizes its parameters and part opacities.
    /// This has to be called before accessing the drawables.
    #[inline]
    pub fn update(&mut self) {
        unsafe { ffi::csmUpdateModel(self.mem.as_ptr()) };
        unsafe { ffi::csmResetDrawableDynamicFlags(self.mem.as_ptr()) };
    }

    /// Returns information about this models size, origin and pixels-per-unit.
    pub fn canvas_info(&self) -> ([f32; 2], [f32; 2], f32) {
        let mut size = [0.0; 2];
        let mut origin = [0.0; 2];
        let mut ppu = 0.0;
        unsafe {
            ffi::csmReadCanvasInfo(
                self.mem.as_ptr(),
                &mut size as *mut _ as *mut _,
                &mut origin as *mut _ as *mut _,
                &mut ppu,
            );
        }
        (size, origin, ppu)
    }

    /// Returns the number of drawables of this model.
    #[inline]
    pub fn drawable_count(&self) -> usize {
        self.drawable_count
    }

    /// Returns the render orders of the drawables.
    #[inline]
    pub fn drawable_render_orders(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableRenderOrders(self.as_ptr()),
                self.drawable_count,
            )
        }
    }

    /// Returns the draw orders of the drawables.
    #[inline]
    pub fn drawable_draw_orders(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableDrawOrders(self.as_ptr()),
                self.drawable_count,
            )
        }
    }

    /// Returns the texture indices of the drawables.
    #[inline]
    pub fn drawable_texture_indices(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableTextureIndices(self.as_ptr()),
                self.drawable_count,
            )
        }
    }

    /// Returns the number of indices for every drawable.
    #[inline]
    fn drawable_index_counts(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableIndexCounts(self.as_ptr()),
                self.drawable_count,
            )
        }
    }

    /// Returns the indices of the drawable at the specified index.
    #[inline]
    pub fn drawable_indices(&self, idx: usize) -> &[u16] {
        unsafe {
            slice::from_raw_parts(
                *ffi::csmGetDrawableIndices(self.as_ptr()).add(idx),
                self.drawable_index_counts()[idx] as usize,
            )
        }
    }

    /// Returns the number of vertices of this model.
    #[inline]
    pub fn drawable_vertex_counts(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableVertexCounts(self.as_ptr()),
                self.drawable_count,
            )
        }
    }

    /// Returns the vertex positions of the drawable at the specified index.
    #[inline]
    pub fn drawable_vertex_positions(&self, idx: usize) -> &[[f32; 2]] {
        unsafe {
            slice::from_raw_parts(
                *ffi::csmGetDrawableVertexPositions(self.as_ptr()).add(idx) as *const _,
                self.drawable_vertex_counts()[idx] as usize,
            )
        }
    }

    /// Returns the uv coordinates of the drawable at the specified index.
    #[inline]
    pub fn drawable_vertex_uvs(&self, idx: usize) -> &[[f32; 2]] {
        unsafe {
            slice::from_raw_parts(
                *ffi::csmGetDrawableVertexUvs(self.as_ptr()).add(idx) as *const _,
                self.drawable_vertex_counts()[idx] as usize,
            )
        }
    }

    /// Returns the drawable opacities.
    #[inline]
    pub fn drawable_opacities(&self) -> &[f32] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableOpacities(self.as_ptr()),
                self.drawable_count,
            )
        }
    }

    #[inline]
    fn drawable_mask_counts(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableMaskCounts(self.as_ptr()),
                self.drawable_count,
            )
        }
    }

    /// Returns the mask of the drawable at the specified index.
    #[inline]
    pub fn drawable_masks(&self, idx: usize) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                slice::from_raw_parts(ffi::csmGetDrawableMasks(self.as_ptr()), self.drawable_count)
                    [idx] as *const _,
                self.drawable_mask_counts()[idx] as usize,
            )
        }
    }

    /// Returns true if this model is masked.
    #[inline]
    pub fn is_masked(&self) -> bool {
        self.drawable_mask_counts().iter().any(|c| *c <= 0)
    }

    /// Returns the [ConstantFlags](./struct.ConstantFlags.html).
    #[inline]
    pub fn drawable_constant_flags(&self) -> &[ConstantFlags] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableConstantFlags(self.as_ptr()) as *const ConstantFlags,
                self.drawable_count,
            )
        }
    }

    /// Returns the [DynamicFlags](./struct.DynamicFlags.html).
    #[inline]
    pub fn drawable_dynamic_flags(&self) -> &[DynamicFlags] {
        unsafe {
            slice::from_raw_parts(
                ffi::csmGetDrawableDynamicFlags(self.as_ptr()) as *const DynamicFlags,
                self.drawable_count,
            )
        }
    }

    /// Returns a reference to the underlying [Moc](./struct.Moc.html).
    #[inline]
    pub fn moc(&self) -> &Moc {
        &self.moc
    }

    /// Returns the raw
    /// [csmModel](../cubism_core_sys/model/struct.csmModel.html) ptr.
    #[inline]
    pub(in crate) fn as_ptr(&self) -> *mut csmModel {
        self.mem.as_ptr()
    }

    #[inline]
    pub fn parameters(&self) -> ParameterIter {
        ParameterIter {
            model: self,
            idx: 0,
        }
    }

    #[inline]
    pub fn parameters_mut(&mut self) -> ParameterIterMut {
        ParameterIterMut {
            model: self,
            idx: 0,
        }
    }

    #[inline]
    pub fn parts(&self) -> PartIter {
        PartIter {
            model: self,
            idx: 0,
        }
    }

    #[inline]
    pub fn parts_mut(&mut self) -> PartIterMut {
        PartIterMut {
            model: self,
            idx: 0,
        }
    }
}

impl Model {
    unsafe fn new_impl(moc: Rc<Moc>, mem: AlignedMemory<ffi::csmModel>) -> Model {
        let param_values = NonNull::from(slice::from_raw_parts_mut(
            ffi::csmGetParameterValues(mem.as_ptr()),
            moc.parameter_count(),
        ));
        let part_opacities = NonNull::from(slice::from_raw_parts_mut(
            ffi::csmGetPartOpacities(mem.as_ptr()),
            moc.part_count(),
        ));
        let drawable_count = ffi::csmGetDrawableCount(mem.as_ptr()) as usize;

        Model {
            mem,
            moc,
            param_val: param_values,
            part_opacities,
            drawable_count,
        }
    }
}

impl Clone for Model {
    fn clone(&self) -> Self {
        let model_mem = unsafe { self.moc.init_new_model() };
        let mut model = unsafe { Self::new_impl(self.moc.clone(), model_mem) };
        model
            .parameter_values_mut()
            .copy_from_slice(self.parameter_values());
        model
            .part_opacities_mut()
            .copy_from_slice(self.part_opacities());
        model
    }
}

impl ops::Deref for Model {
    type Target = Moc;
    fn deref(&self) -> &Self::Target {
        &self.moc
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Parameter<'model> {
    pub id: &'model str,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
}

#[derive(Debug)]
pub struct ParameterMut<'model> {
    pub id: &'model str,
    pub value: &'model mut f32,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct Part<'model> {
    pub id: &'model str,
    pub opacity: f32,
}

#[derive(Debug)]
pub struct PartMut<'model> {
    pub id: &'model str,
    pub opacity: &'model mut f32,
}

pub struct ParameterIter<'model> {
    model: &'model Model,
    idx: usize,
}

impl<'model> iter::ExactSizeIterator for ParameterIter<'model> {}
impl<'model> iter::FusedIterator for ParameterIter<'model> {}
impl<'model> Iterator for ParameterIter<'model> {
    type Item = Parameter<'model>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.model.parameter_count() {
            let param = self.model.parameter_at(self.idx);
            self.idx += 1;
            Some(param)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.model.parameter_count() - self.idx;
        (len, Some(len))
    }
}

pub struct ParameterIterMut<'model> {
    model: &'model mut Model,
    idx: usize,
}

impl<'model> iter::ExactSizeIterator for ParameterIterMut<'model> {}
impl<'model> iter::FusedIterator for ParameterIterMut<'model> {}
impl<'model> Iterator for ParameterIterMut<'model> {
    type Item = ParameterMut<'model>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.model.parameter_count() {
            // safety: transmuting the lifetimes is safe here, since we only create mutable
            // borrows to disjoint objects
            let part = unsafe { mem::transmute(self.model.parameter_at_mut(self.idx)) };
            self.idx += 1;
            Some(part)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.model.parameter_count() - self.idx;
        (len, Some(len))
    }
}

pub struct PartIter<'model> {
    model: &'model Model,
    idx: usize,
}

impl<'model> iter::ExactSizeIterator for PartIter<'model> {}
impl<'model> iter::FusedIterator for PartIter<'model> {}
impl<'model> Iterator for PartIter<'model> {
    type Item = Part<'model>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.model.part_count() {
            let part = self.model.part_at(self.idx);
            self.idx += 1;
            Some(part)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.model.part_count() - self.idx;
        (len, Some(len))
    }
}

pub struct PartIterMut<'model> {
    model: &'model mut Model,
    idx: usize,
}

impl<'model> iter::ExactSizeIterator for PartIterMut<'model> {}
impl<'model> iter::FusedIterator for PartIterMut<'model> {}
impl<'model> Iterator for PartIterMut<'model> {
    type Item = PartMut<'model>;

    fn next(&mut self) -> Option<PartMut<'model>> {
        if self.idx < self.model.part_count() {
            // safety: transmuting the lifetimes is safe here, since we only create mutable
            // borrows to disjoint objects
            let part = unsafe { mem::transmute(self.model.part_at_mut(self.idx)) };
            self.idx += 1;
            Some(part)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.model.part_count() - self.idx;
        (len, Some(len))
    }
}
