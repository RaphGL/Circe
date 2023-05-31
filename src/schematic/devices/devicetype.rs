use euclid::Vector2D;
use iced::{Size, widget::canvas::{self, stroke, LineCap, path::Builder, LineDash}, Color};

use crate::{
    transforms::{
        SSPoint, VSBox, VSPoint, VCTransform, Point, ViewportSpace, SSBox
    }, schematic::Drawable, 
};
use iced::{widget::canvas::{Frame, Stroke}};
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Port {
    pub name: &'static str,
    pub offset: SSPoint,
}

impl Drawable for Port {
    fn draw_persistent(&self, vct: VCTransform, vcscale: f32, frame: &mut iced::widget::canvas::Frame) {
        let f = canvas::Fill {
            style: canvas::Style::Solid(Color::from_rgba(1.0, 0.0, 0.0, 1.0)),
            ..canvas::Fill::default()
        };
        let dim = 0.4;
        let ssb = VSBox::new(
            self.offset.cast::<f32>().cast_unit() - Vector2D::new(dim/2.0, dim/2.0), 
            self.offset.cast::<f32>().cast_unit() + Vector2D::new(dim/2.0, dim/2.0), 
        );

        let csbox = vct.outer_transformed_box(&ssb);
        
        let top_left = csbox.min;
        let size = Size::new(csbox.width(), csbox.height());
        frame.fill_rectangle(Point::from(top_left).into(), size, f);
    }

    fn draw_selected(&self, vct: crate::transforms::VCTransform, vcscale: f32, frame: &mut iced::widget::canvas::Frame) {
        let stroke = Stroke {
            width: (STROKE_WIDTH * vcscale).max(STROKE_WIDTH * 1.),
            style: stroke::Style::Solid(Color::from_rgb(1.0, 1.0, 0.0)),
            line_cap: LineCap::Square,
            ..Stroke::default()
        };
        let mut path_builder = Builder::new();
        let dim = 0.4;
        let vsb = VSBox::new(
            self.offset.cast::<f32>().cast_unit() - Vector2D::new(dim/2.0, dim/2.0), 
            self.offset.cast::<f32>().cast_unit() + Vector2D::new(dim/2.0, dim/2.0), 
        );
        let csb = vct.outer_transformed_box(&vsb);
        let size = Size::new(csb.width(), csb.height());
        path_builder.rectangle(Point::from(csb.min).into(), size);
        frame.stroke(&path_builder.build(), stroke);     
    }

    fn draw_preview(&self, vct: crate::transforms::VCTransform, vcscale: f32, frame: &mut iced::widget::canvas::Frame) {
        let stroke = Stroke {
            width: (STROKE_WIDTH * vcscale).max(STROKE_WIDTH * 1.),
            style: stroke::Style::Solid(Color::from_rgb(1.0, 1.0, 0.5)),
            line_cap: LineCap::Square,
            ..Stroke::default()
        };
        let mut path_builder = Builder::new();
        let dim = 0.4;
        let vsb = VSBox::new(
            self.offset.cast::<f32>().cast_unit() - Vector2D::new(dim/2.0, dim/2.0), 
            self.offset.cast::<f32>().cast_unit() + Vector2D::new(dim/2.0, dim/2.0), 
        );
        let csb = vct.outer_transformed_box(&vsb);
        let size = Size::new(csb.width(), csb.height());
        path_builder.rectangle(Point::from(csb.min).into(), size);
        frame.stroke(&path_builder.build(), stroke);     
    }
}

const STROKE_WIDTH: f32 = 0.1;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graphics <T> {
    // T is just an identifier so the graphic is not used for the wrong device type, analogous to ViewportSpace/SchematicSpace of euclid
    pts: Vec<Vec<VSPoint>>,
    ports: Vec<Port>,
    bounds: SSBox,
    marker: core::marker::PhantomData<T>,
}
impl<T> Graphics<T> {
    pub fn bounds(&self) -> &SSBox {
        &self.bounds
    }
    pub fn ports(&self) -> &[Port] {
        &self.ports
    }
    pub fn default_r() -> Self {
        Graphics { 
            pts: vec![
                vec![
                    VSPoint::new(0., 3.),
                    VSPoint::new(0., -3.),
                ],
                vec![
                    VSPoint::new(-1., 2.),
                    VSPoint::new(-1., -2.),
                    VSPoint::new(1., -2.),
                    VSPoint::new(1., 2.),
                    VSPoint::new(-1., 2.),
                ],
            ],
            ports: vec![
                Port {name: "+", offset: SSPoint::new(0, 3)},
                Port {name: "-", offset: SSPoint::new(0, -3)},
            ], 
            bounds: SSBox::new(SSPoint::new(-2, 3), SSPoint::new(2, -3)), 
            marker: core::marker::PhantomData 
        }
    }
    pub fn default_gnd() -> Self {
        Graphics { 
            pts: vec![
                vec![
                    VSPoint::new(0., 2.),
                    VSPoint::new(0., -1.)
                ],
                vec![
                    VSPoint::new(0., -2.),
                    VSPoint::new(1., -1.),
                    VSPoint::new(-1., -1.),
                    VSPoint::new(0., -2.),
                ],
            ],
            ports: vec![
                Port {name: "gnd", offset: SSPoint::new(0, 2)}
            ], 
            bounds: SSBox::new(SSPoint::new(-1, 2), SSPoint::new(1, -2)), 
            marker: core::marker::PhantomData 
        }
    }
    pub fn stroke_bounds(&self, vct_composite: VCTransform, frame: &mut Frame, stroke: Stroke) {
        let mut path_builder = Builder::new();
        let vsb = self.bounds.cast().cast_unit();
        let csb = vct_composite.outer_transformed_box(&vsb);
        let size = Size::new(csb.width(), csb.height());
        path_builder.rectangle(Point::from(csb.min).into(), size);
        frame.stroke(&path_builder.build(), stroke);    
    }
    pub fn stroke_symbol(&self, vct_composite: VCTransform, frame: &mut Frame, stroke: Stroke) {
        // let mut path_builder = Builder::new();
        for v1 in &self.pts {
            // there's a bug where dashed stroke can draw a solid line across a move
            // path_builder.move_to(Point::from(vct_composite.transform_point(v1[0])).into());
            let mut path_builder = Builder::new();
            for v0 in v1 {
                path_builder.line_to(Point::from(vct_composite.transform_point(*v0)).into());
            }
            frame.stroke(&path_builder.build(), stroke.clone());
        }
    }
}
impl <T> Drawable for Graphics<T> {
    fn draw_persistent(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        let stroke = Stroke {
            width: (STROKE_WIDTH * vcscale).max(STROKE_WIDTH * 2.0),
            style: stroke::Style::Solid(Color::from_rgb(0.0, 0.8, 0.0)),
            line_cap: LineCap::Square,
            ..Stroke::default()
        };
        // self.stroke_bounds(vct, frame, stroke.clone());
        self.stroke_symbol(vct, frame, stroke.clone());
        for p in &self.ports {
            p.draw_persistent(vct, vcscale, frame)
        }
    }
    fn draw_selected(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        let stroke = Stroke {
            width: (STROKE_WIDTH * vcscale).max(STROKE_WIDTH * 2.) / 2.0,
            style: stroke::Style::Solid(Color::from_rgb(1.0, 0.8, 0.0)),
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        self.stroke_bounds(vct, frame, stroke.clone());
        // self.stroke_ports(vct, frame, stroke.clone());
        self.stroke_symbol(vct, frame, stroke.clone());
        for p in &self.ports {
            p.draw_selected(vct, vcscale, frame)
        }
    }
    fn draw_preview(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        let stroke = Stroke {
            width: (STROKE_WIDTH * vcscale).max(STROKE_WIDTH * 1.) / 2.0,
            style: stroke::Style::Solid(Color::from_rgb(1.0, 1.0, 0.5)),
            line_cap: LineCap::Butt,
            line_dash: LineDash{segments: &[3. * (STROKE_WIDTH * vcscale).max(STROKE_WIDTH * 2.0)], offset: 0},
            ..Stroke::default()
        };
        self.stroke_bounds(vct, frame, stroke.clone());
        self.stroke_symbol(vct, frame, stroke.clone());
        for p in &self.ports {
            p.draw_preview(vct, vcscale, frame)
        }
    }
}