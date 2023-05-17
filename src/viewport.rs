/// the viewport handles visual transforms from the schematic to canvas and vice-versa
/// CanvasSpace <-> ViewportSpace <-> SchematicSpace 
/// CanvasSpace is the UI canvas coordinate
/// ViewportSpace is the schematic coordinate in f32
/// SchematicSpace is the schematic coordinate in i16

use crate::transforms::{CSPoint, VSPoint, SSPoint, VCTransform, CVTransform, CanvasSpace, ViewportSpace, VSBox, CSBox};
use crate::schematic::Schematic;

use euclid::{Vector2D, Box2D};

use iced::widget::canvas::{
    stroke, Cache, Cursor, Geometry, LineCap, Path, Stroke, LineDash, Frame,
};

use iced::{Color};

#[derive(Clone, Debug)]
pub enum ViewportState {
    Panning,
    Selecting(VSPoint),
    NewView(VSPoint, VSPoint),
    None,
}

impl Default for ViewportState {
    fn default() -> Self {
        ViewportState::None
    }
}

pub struct Viewport {
    pub schematic: Box<Schematic>,
    pub state: ViewportState,
    transform: VCTransform, 
    scale: f32,

    curpos: Option<(CSPoint, VSPoint, SSPoint)>,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport { 
            schematic: Box::<Schematic>::default(),
            state: Default::default(),
            transform: VCTransform::default().pre_scale(10., 10.), 
            scale: 10.0,  // scale from canvas to viewport, sqrt of transform determinant. Save value to save computing power

            curpos: None,
        }
    }
}

impl Viewport {
    const MAX_SCALING: f32 = 100.0;  // most zoomed in - every 100 pixel is 1
    const MIN_SCALING: f32 = 1.;  // most zoomed out - every pixel is 1

    pub fn curpos_ssp(&self) -> Option<SSPoint> {
        self.curpos.map(|tup| tup.2)
    }

    pub fn cv_transform(&self) -> CVTransform {
        self.transform.inverse().unwrap()
    }

    pub fn vc_transform(&self) -> VCTransform {
        self.transform
    }
    
    pub fn vc_scale(&self) -> f32 {
        self.scale
    }

    pub fn cv_scale(&self) -> f32 {
        1. / self.scale
    }

    pub fn display_bounds(&mut self, csb: CSBox, vsb: VSBox) {  // change transform such that VSBox fit inside CSBox
        let mut vct = VCTransform::identity();
        
        let s = (csb.height() / vsb.height()).min(csb.height() / vsb.height()).clamp(Viewport::MIN_SCALING, Viewport::MAX_SCALING);  // scale from vsb to fit inside csb
        vct = vct.then_scale(s, s);

        let v = csb.center() - vct.transform_point(vsb.center());  // vector from vsb to csb
        vct = vct.then_translate(v);

        self.transform = vct;
        self.scale = s;

        // recalculate cursor in viewport, or it will be wrong until cursor is moved
        if let Some((csp, ..)) = self.curpos {
            self.curpos_update(Some(csp));
        }
    }

    pub fn curpos_update(&mut self, opt_csp: Option<CSPoint>) {
        if let Some(csp1) = opt_csp {
            let vsp1 = self.cv_transform().transform_point(csp1);
            let ssp1: SSPoint = vsp1.round().cast().cast_unit();
            match &mut self.state {
                ViewportState::Panning => {
                    if let Some((csp0, vsp0, ssp0)) = self.curpos {
                        let v = (csp1 - csp0).cast_unit() / self.scale;
                        self.transform = self.vc_transform().pre_translate(v);
                    }
                },
                ViewportState::NewView(vsp_origin, vsp_other) => {
                    if (*vsp_origin - vsp1).length() > 10. {
                        *vsp_other = vsp1; 
                    } else {
                        *vsp_other = *vsp_origin; 
                    }
                }
                ViewportState::Selecting(vsp0) => {
                    // todo
                },
                ViewportState::None => {
                    // todo?
                },
            }

            self.curpos = Some((csp1, vsp1, ssp1));
            self.schematic.curpos_update(Some((vsp1, ssp1)));
        } else {
            self.curpos = None;
            self.schematic.curpos_update(None);
        }
    }

    pub fn zoom(&mut self, scale: f32) {
        if let Some((csp, vsp, _)) = self.curpos {
            let scaled_transform = self.transform.then_scale(scale, scale);

            let mut new_transform;  // transform with applied scale and translated to maintain p_viewport position
            let scaled_determinant = scaled_transform.determinant();
            if scaled_determinant < Viewport::MIN_SCALING * Viewport::MIN_SCALING {  // minimum scale
                let clamped_scale = Viewport::MIN_SCALING / (self.scale);
                new_transform = self.transform.then_scale(clamped_scale, clamped_scale);
            } else if scaled_determinant <= Viewport::MAX_SCALING * Viewport::MAX_SCALING {  // adjust scale
                new_transform = scaled_transform;
            } else {  // maximum scale
                let clamped_scale = Viewport::MAX_SCALING / (self.scale);
                new_transform = self.transform.then_scale(clamped_scale, clamped_scale);
            }
    
            let csp1 = new_transform.transform_point(vsp);
            let translation = csp - csp1;
            new_transform = new_transform.then_translate(translation);
    
            self.transform = new_transform;
            self.scale = self.transform.determinant().sqrt();
        }
    }

    pub fn draw_cursor(&self, frame: &mut Frame) {
        if let Some((_csp, _vsp, ssp)) = self.curpos {
            let cursor_stroke = || -> Stroke {
                Stroke {
                    width: 1.0,
                    style: stroke::Style::Solid(Color::from_rgb(1.0, 0.9, 0.0)),
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                }
            };
            let curdim = 5.0;
            let csp = self.vc_transform().transform_point(ssp.cast().cast_unit());
            let csp_topleft = csp - Vector2D::from([curdim/2.; 2]);
            let s = iced::Size::from([curdim, curdim]);
            let c = Path::rectangle(iced::Point::from([csp_topleft.x, csp_topleft.y]), s);
            frame.stroke(&c, cursor_stroke());
        }
    }

    pub fn draw_grid(&self, frame: &mut Frame, bb_viewport: Box2D<f32, CanvasSpace>) {
        fn draw_grid_w_spacing(spacing: f32, bb_canvas: Box2D<f32, ViewportSpace>, cvtransform: VCTransform, frame: &mut Frame, stroke: Stroke) {
            let v = bb_canvas.max - bb_canvas.min;
            for col in 0..=(v.x.ceil() / spacing) as u32 {
                for row in 0..=(v.y.ceil() / spacing) as u32 {
                    let p_c = bb_canvas.min + Vector2D::<f32, ViewportSpace>::from([col as f32 * spacing, row as f32 * spacing]);
                    let p = cvtransform.transform_point(p_c);
                    let p = iced::Point::from([p.x, p.y]);
                    let c = Path::line(p, p);
                    frame.stroke(&c, stroke.clone());
                }
            }
        }
        let coarse_grid_threshold: f32 = 2.0;
        let fine_grid_threshold: f32 = 4.;
        if self.scale > coarse_grid_threshold {
            // draw coarse grid
            let spacing = 16.;
            let bb_canvas = VSBox::new(
                (self.cv_transform().transform_point(bb_viewport.min) / spacing).round() * spacing,
                (self.cv_transform().transform_point(bb_viewport.max) / spacing).round() * spacing,
            );

            let grid_stroke = Stroke {
                width: (0.5 * self.scale).clamp(0.5, 3.0),
                style: stroke::Style::Solid(Color::WHITE),
                line_cap: LineCap::Round,
                ..Stroke::default()
            };

            draw_grid_w_spacing(
                spacing, 
                bb_canvas, 
                self.vc_transform(), 
                frame, 
                grid_stroke,
            );

            if self.scale > fine_grid_threshold {  // draw fine grid if sufficiently zoomed in
                let spacing = 2.;
                let bb_canvas = VSBox::new(
                    (self.cv_transform().transform_point(bb_viewport.min) / spacing).round() * spacing,
                    (self.cv_transform().transform_point(bb_viewport.max) / spacing).round() * spacing,
                );
        
                let grid_stroke = Stroke {
                    width: 1.,
                    style: stroke::Style::Solid(Color::WHITE),
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                };
        
                draw_grid_w_spacing(
                    spacing, 
                    bb_canvas, 
                    self.vc_transform(), 
                    frame, 
                    grid_stroke,
                );
            } 
        }
        let ref_stroke = Stroke {
            width: (0.5 * self.scale).clamp(0.5, 3.0),
            style: stroke::Style::Solid(Color::WHITE),
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        let p = self.vc_transform().transform_point(VSPoint::from([0.,0.]));
        let r = self.transform.determinant().sqrt() * 8.;
        let c = Path::circle(iced::Point::from([p.x, p.y]), r);
        frame.stroke(&c, ref_stroke);
    }
}