use cosmic_text::{FamilyOwned, Weight};
use std::any::{Any, TypeId};
use std::ops::Range;

use femtovg::{ImageId, Paint, Path};
use fnv::FnvHashMap;
use morphorm::Units;

use crate::cache::{BoundingBox, CachedData};
use crate::events::ViewHandler;
use crate::prelude::*;
use crate::resource::ResourceManager;
use crate::state::ModelDataStore;
use crate::style::Style;
use crate::text::TextContext;
use vizia_input::{Modifiers, MouseState};
use vizia_storage::SparseSet;
use vizia_style::{
    BoxShadow, Gradient, HorizontalPositionKeyword, Length, LengthOrPercentage, LengthValue,
    LineDirection, VerticalPositionKeyword,
};

/// Cached data used for drawing.
pub struct DrawCache {
    pub shadow_image: SparseSet<(ImageId, ImageId)>,
    pub text_lines: SparseSet<Vec<(Range<usize>, femtovg::TextMetrics)>>,
}

impl DrawCache {
    pub fn new() -> Self {
        Self { shadow_image: SparseSet::new(), text_lines: SparseSet::new() }
    }

    pub fn remove(&mut self, entity: Entity) {
        self.shadow_image.remove(entity);
        self.text_lines.remove(entity);
    }
}

/// A restricted context used when drawing.
pub struct DrawContext<'a> {
    pub(crate) current: Entity,
    pub captured: &'a Entity,
    pub focused: &'a Entity,
    pub hovered: &'a Entity,
    pub style: &'a Style,
    pub cache: &'a CachedData,
    pub draw_cache: &'a mut DrawCache,
    pub tree: &'a Tree<Entity>,
    pub(crate) data: &'a SparseSet<ModelDataStore>,
    pub views: &'a FnvHashMap<Entity, Box<dyn ViewHandler>>,
    pub resource_manager: &'a ResourceManager,
    pub text_context: &'a mut TextContext,
    pub modifiers: &'a Modifiers,
    pub mouse: &'a MouseState<Entity>,
}

macro_rules! style_getter_units {
    ($name:ident) => {
        pub fn $name(&self) -> Units {
            let result = self.style.$name.get(self.current);
            if let Some(Units::Pixels(p)) = result {
                Units::Pixels(self.logical_to_physical(*p))
            } else {
                result.copied().unwrap_or_default()
            }
        }
    };
}

macro_rules! get_property {
    ($ty:ty, $name:ident) => {
        pub fn $name(&self) -> $ty {
            self.style.$name.get(self.current).copied().unwrap_or_default()
        }
    };
}

macro_rules! get_color_property {
    ($ty:ty, $name:ident) => {
        pub fn $name(&self) -> $ty {
            let opacity = self.cache.get_opacity(self.current);
            if let Some(col) = self.style.$name.get(self.current) {
                Color::rgba(col.r(), col.g(), col.b(), (opacity * col.a() as f32) as u8)
            } else {
                Color::rgba(0, 0, 0, 0)
            }
        }
    };
}

macro_rules! get_length_property {
    ($name:ident) => {
        pub fn $name(&self) -> f32 {
            if let Some(length) = self.style.$name.get(self.current) {
                let bounds = self.bounds();

                let px = length.to_pixels(bounds.w.min(bounds.h));
                return self.logical_to_physical(px).round();
            }

            0.0
        }
    };
}

impl<'a> DrawContext<'a> {
    /// Creates a new `DrawContext` from the given `Context`.
    pub fn new(cx: &'a mut Context) -> Self {
        Self {
            current: cx.current,
            captured: &cx.captured,
            focused: &cx.focused,
            hovered: &cx.hovered,
            style: &cx.style,
            cache: &mut cx.cache,
            draw_cache: &mut cx.draw_cache,
            tree: &cx.tree,
            data: &cx.data,
            views: &cx.views,
            resource_manager: &cx.resource_manager,
            text_context: &mut cx.text_context,
            modifiers: &cx.modifiers,
            mouse: &cx.mouse,
        }
    }

    pub fn bounds(&self) -> BoundingBox {
        self.cache.get_bounds(self.current)
    }

    pub fn clip_region(&self) -> BoundingBox {
        self.cache.get_clip_region(self.current)
    }

    /// Returns the lookup pattern to pick the default font.
    pub fn default_font(&self) -> &[FamilyOwned] {
        &self.style.default_font
    }

    /// Returns the font-size of the current entity in physical coordinates.
    pub fn font_size(&self, entity: Entity) -> f32 {
        self.logical_to_physical(
            self.style.font_size.get(entity).copied().map(|f| f.0).unwrap_or(16.0),
        )
    }

    /// Function to convert logical points to physical pixels.
    pub fn logical_to_physical(&self, logical: f32) -> f32 {
        logical * self.style.dpi_factor as f32
    }

    /// Function to convert physical pixels to logical points.
    pub fn physical_to_logical(&self, physical: f32) -> f32 {
        physical * self.style.dpi_factor as f32
    }

    get_length_property!(border_width);
    get_length_property!(outline_width);
    get_length_property!(outline_offset);
    get_length_property!(border_top_left_radius);
    get_length_property!(border_top_right_radius);
    get_length_property!(border_bottom_left_radius);
    get_length_property!(border_bottom_right_radius);

    pub fn border_top_left_shape(&self) -> BorderCornerShape {
        self.style.border_top_left_shape.get(self.current).copied().unwrap_or_default()
    }

    pub fn border_top_right_shape(&self) -> BorderCornerShape {
        self.style.border_top_right_shape.get(self.current).copied().unwrap_or_default()
    }

    pub fn border_bottom_left_shape(&self) -> BorderCornerShape {
        self.style.border_bottom_left_shape.get(self.current).copied().unwrap_or_default()
    }

    pub fn border_bottom_right_shape(&self) -> BorderCornerShape {
        self.style.border_bottom_right_shape.get(self.current).copied().unwrap_or_default()
    }

    // style_getter_untranslated!(LengthOrPercentage, border_width);
    // style_getter_untranslated!(LengthOrPercentage, border_top_right_radius);
    // style_getter_untranslated!(LengthOrPercentage, border_top_left_radius);
    // style_getter_untranslated!(LengthOrPercentage, border_bottom_right_radius);
    // style_getter_untranslated!(LengthOrPercentage, border_bottom_left_radius);
    // style_getter_untranslated!(LengthOrPercentage, outline_width);
    // style_getter_untranslated!(LengthOrPercentage, outline_offset);
    // style_getter_untranslated!(LengthOrPercentage, outer_shadow_h_offset);
    // style_getter_untranslated!(LengthOrPercentage, outer_shadow_v_offset);
    // style_getter_untranslated!(LengthOrPercentage, outer_shadow_blur);
    // style_getter_untranslated!(LengthOrPercentage, inner_shadow_h_offset);
    // style_getter_untranslated!(LengthOrPercentage, inner_shadow_v_offset);
    // style_getter_untranslated!(LengthOrPercentage, inner_shadow_blur);
    style_getter_units!(child_left);
    style_getter_units!(child_right);
    style_getter_units!(child_top);
    style_getter_units!(child_bottom);
    get_color_property!(Color, background_color);
    // get_color_property!(Color, font_color);
    get_color_property!(Color, border_color);
    get_color_property!(Color, outline_color);
    // style_getter_untranslated!(Color, outer_shadow_color);
    // style_getter_untranslated!(Color, inner_shadow_color);
    get_color_property!(Color, selection_color);
    get_color_property!(Color, caret_color);
    // style_getter_untranslated!(LinearGradient, background_gradient);
    // style_getter_untranslated!(BorderCornerShape, border_top_right_shape);
    // style_getter_untranslated!(BorderCornerShape, border_top_left_shape);
    // style_getter_untranslated!(BorderCornerShape, border_bottom_right_shape);
    // style_getter_untranslated!(BorderCornerShape, border_bottom_left_shape);
    // style_getter_untranslated!(String, background_image);
    // style_getter_untranslated!(String, text);
    // get_property!(String, image);
    // style_getter_untranslated!(String, font);
    // get_property!(bool, text_wrap);

    pub fn font_color(&self) -> Color {
        let opacity = self.cache.get_opacity(self.current);
        if let Some(col) = self.style.font_color.get(self.current) {
            Color::rgba(col.r(), col.g(), col.b(), (opacity * col.a() as f32) as u8)
        } else {
            Color::rgba(0, 0, 0, 255)
        }
    }

    pub fn text_wrap(&self) -> bool {
        self.style.text_wrap.get(self.current).copied().unwrap_or(true)
    }

    // pub fn font(&self) -> Option<&String> {
    //     self.style.font.get(self.current)
    // }

    pub fn image(&self) -> Option<&String> {
        self.style.image.get(self.current)
    }

    pub fn box_shadows(&self) -> Option<&Vec<BoxShadow>> {
        self.style.box_shadow.get(self.current)
    }

    // pub fn text(&self) -> Option<&String> {
    //     self.style.text.get(self.current)
    // }

    pub fn opacity(&self) -> f32 {
        self.cache.get_opacity(self.current)
    }

    pub fn scale_factor(&self) -> f32 {
        self.style.dpi_factor as f32
    }

    pub fn draw_shadows(&mut self, canvas: &mut Canvas, path: &mut Path) {
        if let Some(box_shadows) = self.box_shadows() {
            for box_shadow in box_shadows.iter().rev() {
                // Create a shadow image
                // Draw the path to the shadow image
                // Blur the shadow image
                // Draw the shadow image onto the canvas
                let color = box_shadow.color.unwrap_or_default();
                let x_offset = box_shadow.x_offset.to_px().unwrap_or(0.0) * self.scale_factor();
                let y_offset = box_shadow.y_offset.to_px().unwrap_or(0.0) * self.scale_factor();
                // canvas.save();
                // canvas.translate(x_offset, y_offset);
                // canvas.fill_path(path, &femtovg::Paint::color(color.into()));
                // canvas.restore();

                let blur_radius =
                    box_shadow.blur_radius.as_ref().and_then(|br| br.to_px()).unwrap_or(0.0);
                let sigma = blur_radius / 2.0;
                let d = (sigma * 5.0).ceil();

                let bounds = self.bounds();
                // println!("bounds: {}", bounds);

                let (source, target) = {
                    (
                        canvas
                            .create_image_empty(
                                (bounds.w + d) as usize,
                                (bounds.h + d) as usize,
                                femtovg::PixelFormat::Rgba8,
                                femtovg::ImageFlags::FLIP_Y | femtovg::ImageFlags::PREMULTIPLIED,
                            )
                            .unwrap(),
                        canvas
                            .create_image_empty(
                                (bounds.w + d) as usize,
                                (bounds.h + d) as usize,
                                femtovg::PixelFormat::Rgba8,
                                femtovg::ImageFlags::FLIP_Y | femtovg::ImageFlags::PREMULTIPLIED,
                            )
                            .unwrap(),
                    )
                };

                canvas.save();
                canvas.set_render_target(femtovg::RenderTarget::Image(source));
                canvas.reset_scissor();
                canvas.reset_transform();
                canvas.clear_rect(
                    0,
                    0,
                    (bounds.w + d) as u32,
                    (bounds.h + d) as u32,
                    femtovg::Color::rgba(0, 0, 0, 0),
                );
                canvas.translate(-bounds.x + d / 2.0, -bounds.y + d / 2.0);
                let paint = Paint::color(color.into());
                canvas.fill_path(&mut path.clone(), &paint);
                canvas.restore();

                let target_image = if blur_radius > 0.0 {
                    canvas.filter_image(
                        target,
                        femtovg::ImageFilter::GaussianBlur { sigma },
                        source,
                    );
                    target
                } else {
                    source
                };

                canvas.set_render_target(femtovg::RenderTarget::Screen);
                canvas.save();
                canvas.translate(x_offset, y_offset);
                let mut shadow_path = Path::new();
                shadow_path.rect(
                    bounds.x - d / 2.0,
                    bounds.y - d / 2.0,
                    bounds.w + d,
                    bounds.h + d,
                );

                // shadow_path.rect(0.0, 0.0, bounds.w + d, bounds.h + d);

                canvas.fill_path(
                    &mut shadow_path,
                    &Paint::image(
                        target_image,
                        bounds.x - d / 2.0,
                        bounds.y - d / 2.0,
                        bounds.w + d,
                        bounds.h + d,
                        0f32,
                        1f32,
                    ),
                );

                // canvas.fill_path(
                //     &mut shadow_path,
                //     &Paint::image(source, 0.0, 0.0, bounds.w + d, bounds.h + d, 0f32, 1f32),
                // );
                // canvas.fill_path(
                //     &mut shadow_path,
                //     &femtovg::Paint::color(femtovg::Color::rgb(0, 0, 0)),
                // );
                canvas.restore();

                // canvas.delete_image(source);
                // canvas.delete_image(target);
            }
        }
    }

    pub fn draw_gradient(&self, canvas: &mut Canvas, paint: &mut Paint) {
        let bounds = self.bounds();

        let parent = self
            .tree
            .get_layout_parent(self.current)
            .expect(&format!("Failed to find parent somehow: {}", self.current));

        let parent_width = self.cache.get_width(parent);
        let parent_height = self.cache.get_height(parent);

        if let Some(gradient) = self.style.background_gradient.get(self.current) {
            match gradient {
                Gradient::Linear(linear_gradient) => {
                    let (_, _, end_x, end_y, parent_length) = match linear_gradient.direction {
                        LineDirection::Horizontal(horizontal_keyword) => match horizontal_keyword {
                            HorizontalPositionKeyword::Left => {
                                (0.0, 0.0, bounds.w, 0.0, parent_width)
                            }

                            HorizontalPositionKeyword::Right => {
                                (0.0, 0.0, bounds.w, 0.0, parent_width)
                            }
                        },

                        LineDirection::Vertical(vertical_keyword) => match vertical_keyword {
                            VerticalPositionKeyword::Bottom => {
                                (0.0, 0.0, 0.0, bounds.h, parent_height)
                            }

                            VerticalPositionKeyword::Top => {
                                (0.0, 0.0, 0.0, bounds.h, parent_height)
                            }
                        },

                        LineDirection::Corner { horizontal, vertical } => {
                            match (horizontal, vertical) {
                                (
                                    HorizontalPositionKeyword::Right,
                                    VerticalPositionKeyword::Bottom,
                                ) => (0.0, 0.0, bounds.w, bounds.h, parent_width),

                                _ => (0.0, 0.0, 0.0, 0.0, 0.0),
                            }
                        }

                        _ => (0.0, 0.0, 0.0, 0.0, 0.0),
                    };

                    let num_stops = linear_gradient.stops.len();

                    let stops = linear_gradient
                        .stops
                        .iter()
                        .enumerate()
                        .map(|(index, stop)| {
                            let pos = if let Some(pos) = &stop.position {
                                pos.to_pixels(parent_length) / parent_length
                            } else {
                                index as f32 / (num_stops - 1) as f32
                            };
                            let col: femtovg::Color = stop.color.into();
                            (pos, col)
                        })
                        .collect::<Vec<_>>();

                    *paint = Paint::linear_gradient_stops(
                        bounds.x,
                        bounds.y,
                        bounds.x + end_x,
                        bounds.y + end_y,
                        stops.as_slice(),
                    )
                }

                _ => {}
            }
        }
    }

    pub fn draw_text(&mut self, canvas: &mut Canvas, origin: (f32, f32), justify: (f32, f32)) {
        if let Ok(draw_commands) =
            self.text_context.fill_to_cmds(canvas, self.current, origin, justify)
        {
            for (color, cmds) in draw_commands.into_iter() {
                let temp_paint =
                    Paint::color(femtovg::Color::rgba(color.r(), color.g(), color.b(), color.a()));
                canvas.draw_glyph_cmds(cmds, &temp_paint);
            }
        }
    }

    pub fn draw_highlights(
        &mut self,
        canvas: &mut Canvas,
        origin: (f32, f32),
        justify: (f32, f32),
    ) {
        let selection_color = self.selection_color();
        let mut path = Path::new();
        for (x, y, w, h) in self.text_context.layout_selection(self.current, origin, justify) {
            path.rect(x, y, w, h);
        }
        canvas.fill_path(&mut path, &Paint::color(selection_color.into()));
    }

    pub fn draw_caret(
        &mut self,
        canvas: &mut Canvas,
        origin: (f32, f32),
        justify: (f32, f32),
        width: f32,
    ) {
        let caret_color = self.caret_color();
        if let Some((x, y, w, h)) = self.text_context.layout_caret(
            self.current,
            origin,
            justify,
            self.logical_to_physical(width),
        ) {
            let mut path = Path::new();
            path.rect(x, y, w, h);
            canvas.fill_path(&mut path, &Paint::color(caret_color.into()));
        }
    }
}

impl<'a> DataContext for DrawContext<'a> {
    fn data<T: 'static>(&self) -> Option<&T> {
        // return data for the static model
        if let Some(t) = <dyn Any>::downcast_ref::<T>(&()) {
            return Some(t);
        }

        for entity in self.current.parent_iter(&self.tree) {
            if let Some(model_data_store) = self.data.get(entity) {
                if let Some(model) = model_data_store.models.get(&TypeId::of::<T>()) {
                    return model.downcast_ref::<T>();
                }
            }

            if let Some(view_handler) = self.views.get(&entity) {
                if let Some(data) = view_handler.downcast_ref::<T>() {
                    return Some(data);
                }
            }
        }

        None
    }
}
