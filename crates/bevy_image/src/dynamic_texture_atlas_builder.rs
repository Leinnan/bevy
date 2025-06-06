use crate::{Image, TextureAtlasLayout, TextureFormatPixelInfo as _};
use bevy_asset::RenderAssetUsages;
use bevy_math::{URect, UVec2};
use guillotiere::{size2, Allocation, AtlasAllocator};
use thiserror::Error;
use tracing::error;

/// An error produced by [`DynamicTextureAtlasBuilder`] when trying to add a new
/// texture to a [`TextureAtlasLayout`].
#[derive(Debug, Error)]
pub enum DynamicTextureAtlasBuilderError {
    /// Unable to allocate space within the atlas for the new texture
    #[error("Couldn't allocate space to add the image requested")]
    FailedToAllocateSpace,
    /// Attempted to add a texture to an uninitialized atlas
    #[error("cannot add texture to uninitialized atlas texture")]
    UninitializedAtlas,
    /// Attempted to add an uninitialized texture to an atlas
    #[error("cannot add uninitialized texture to atlas")]
    UninitializedSourceTexture,
}

/// Helper utility to update [`TextureAtlasLayout`] on the fly.
///
/// Helpful in cases when texture is created procedurally,
/// e.g: in a font glyph [`TextureAtlasLayout`], only add the [`Image`] texture for letters to be rendered.
pub struct DynamicTextureAtlasBuilder {
    atlas_allocator: AtlasAllocator,
    padding: u32,
}

impl DynamicTextureAtlasBuilder {
    /// Create a new [`DynamicTextureAtlasBuilder`]
    ///
    /// # Arguments
    ///
    /// * `size` - total size for the atlas
    /// * `padding` - gap added between textures in the atlas, both in x axis and y axis
    pub fn new(size: UVec2, padding: u32) -> Self {
        Self {
            atlas_allocator: AtlasAllocator::new(to_size2(size)),
            padding,
        }
    }

    /// Add a new texture to `atlas_layout`.
    ///
    /// It is the user's responsibility to pass in the correct [`TextureAtlasLayout`].
    /// Also, the asset that `atlas_texture_handle` points to must have a usage matching
    /// [`RenderAssetUsages::MAIN_WORLD`].
    ///
    /// # Arguments
    ///
    /// * `atlas_layout` - The atlas layout to add the texture to.
    /// * `texture` - The source texture to add to the atlas.
    /// * `atlas_texture` - The destination atlas texture to copy the source texture to.
    pub fn add_texture(
        &mut self,
        atlas_layout: &mut TextureAtlasLayout,
        texture: &Image,
        atlas_texture: &mut Image,
    ) -> Result<usize, DynamicTextureAtlasBuilderError> {
        let allocation = self.atlas_allocator.allocate(size2(
            (texture.width() + self.padding).try_into().unwrap(),
            (texture.height() + self.padding).try_into().unwrap(),
        ));
        if let Some(allocation) = allocation {
            assert!(
                atlas_texture.asset_usage.contains(RenderAssetUsages::MAIN_WORLD),
                "The atlas_texture image must have the RenderAssetUsages::MAIN_WORLD usage flag set"
            );

            self.place_texture(atlas_texture, allocation, texture)?;
            let mut rect: URect = to_rect(allocation.rectangle);
            rect.max = rect.max.saturating_sub(UVec2::splat(self.padding));
            Ok(atlas_layout.add_texture(rect))
        } else {
            Err(DynamicTextureAtlasBuilderError::FailedToAllocateSpace)
        }
    }

    fn place_texture(
        &mut self,
        atlas_texture: &mut Image,
        allocation: Allocation,
        texture: &Image,
    ) -> Result<(), DynamicTextureAtlasBuilderError> {
        let mut rect = allocation.rectangle;
        rect.max.x -= self.padding as i32;
        rect.max.y -= self.padding as i32;
        let atlas_width = atlas_texture.width() as usize;
        let rect_width = rect.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size();

        let Some(ref mut atlas_data) = atlas_texture.data else {
            return Err(DynamicTextureAtlasBuilderError::UninitializedAtlas);
        };
        let Some(ref data) = texture.data else {
            return Err(DynamicTextureAtlasBuilderError::UninitializedSourceTexture);
        };
        for (texture_y, bound_y) in (rect.min.y..rect.max.y).map(|i| i as usize).enumerate() {
            let begin = (bound_y * atlas_width + rect.min.x as usize) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_data[begin..end].copy_from_slice(&data[texture_begin..texture_end]);
        }
        Ok(())
    }
}

fn to_rect(rectangle: guillotiere::Rectangle) -> URect {
    URect {
        min: UVec2::new(
            rectangle.min.x.try_into().unwrap(),
            rectangle.min.y.try_into().unwrap(),
        ),
        max: UVec2::new(
            rectangle.max.x.try_into().unwrap(),
            rectangle.max.y.try_into().unwrap(),
        ),
    }
}

fn to_size2(vec2: UVec2) -> guillotiere::Size {
    guillotiere::Size::new(vec2.x as i32, vec2.y as i32)
}
