// ex: Vgnd0 net1 0 0
// device Id, net at port, ground net '0', device voltage 0
mod devicetype;
mod deviceinstance;
use devicetype::Graphics;

use std::{rc::Rc, cell::RefCell, hash::Hasher};
use euclid::{Vector2D, Transform2D, Angle};
use iced::widget::canvas::Frame;
use std::hash::Hash;

use crate::transforms::{ViewportSpace, Point, CanvasSpace};

use iced::{widget::canvas::{Stroke, stroke, LineCap, path::Builder, self, LineDash}, Color, Size};

use crate::{
    schematic::nets::{Drawable},
    transforms::{
        SSPoint, VSBox, VCTransform, SchematicSpace, SSBox, VSPoint
    }, 
};

#[derive(Debug, Clone, Copy)]
pub struct Interactable {
    bounds: SSBox,
    tentative: bool,
    selected: bool,
}

impl Interactable {
    fn new() -> Self {
        Interactable { bounds: SSBox::default(), tentative: false, selected: false }
    }
}
#[derive(Debug)]
struct Identifier {
    id_prefix: &'static [char],  // prefix which determines device type in NgSpice
    id: usize,  // avoid changing - otherwise, 
    custom: Option<String>,  // if some, is set by the user - must use this as is for id - if multiple instances have same, both should be highlighted
    // changing the id will break outputs which reference the old id. Otherwise it can be changed
    // 1. how to catch and highlight duplicates
    // 2. how to know id should not be changed (that it is referenced)
}
/*
duplicates:
    create hashset, for every identifier insert. if duplicate, save in second hashset
    every key in second hashset has duplicates
    iterate through devices and highlight every device with id which matches a key in second hashset

immutable identifier:
    abuse rwlock? references take read lock
    if mutation is desired, must acquire write lock - e.g. no read locks. 
 */
impl Identifier {
    pub fn ng_id(&self) -> String {
        let mut ret = String::new();
        for c in self.id_prefix {
            ret.push(*c);
        }
        if let Some(s) = &self.custom {
            ret.push_str(s);
        } else {
            ret.push_str(&format!("{}", self.id));
        }
        ret
    }
    pub fn new_with_ord(ord: usize) -> Self {
        Identifier { id_prefix: &self::PREFIX_R, id: ord, custom: None }
    }
}
impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.ng_id().eq(&other.ng_id())
    }
}
impl Hash for Identifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ng_id().hash(state);
    }
}
const PREFIX_R: [char; 1] = ['R'];

trait DeviceType <T> {
    fn default_graphics() -> Graphics<T>;
}
#[derive(Debug)]
struct R;
impl <T> DeviceType<T> for R {
    fn default_graphics() -> Graphics<T> {
        Graphics::default_r()
    }
}
#[derive(Debug)]
struct Gnd;
impl <T> DeviceType<T> for Gnd {
    fn default_graphics() -> Graphics<T> {
        Graphics::default_gnd()
    }
}
#[derive(Debug)]
struct SingleValue <T> {
    value: f32,
    marker: core::marker::PhantomData<T>,
}
impl <T> SingleValue<T> {
    fn new() -> Self {
        SingleValue { value: 0.0, marker: core::marker::PhantomData }
    }
}
#[derive(Debug)]
enum Param <T> {
    Value(SingleValue<T>),
}
#[derive(Debug)]
struct Device <T> {
    id: Identifier,
    interactable: Interactable,
    transform: Transform2D<i16, SchematicSpace, SchematicSpace>,
    graphics: Rc<Graphics<T>>,  // contains ports, bounds - can be edited, but contents of GraphicsR cannot be edited (from schematic editor)
    params: Param<T>,
}
impl <T> Device<T> {
    pub fn new_with_ord(ord: usize, graphics: Rc<Graphics<T>>) -> Self {
        Device { 
            id: Identifier::new_with_ord(ord), 
            interactable: Interactable::new(), 
            transform: Transform2D::identity(), 
            graphics, 
            params: Param::Value(SingleValue::<T>::new())
        }
    }
}

pub trait DeviceExt: Drawable {
    fn get_interactable(&self) -> Interactable;
    fn get_transform(&self) -> Transform2D<i16, SchematicSpace, SchematicSpace>;
    fn set_tentative(&mut self);
    fn draw_selected_preview(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame);
    fn tentative_by_vsb(&mut self, vsb: &VSBox);
    fn tentatives_to_selected(&mut self);
    fn move_selected(&mut self, ssv: Vector2D<i16, SchematicSpace>);
    fn clear_selected(&mut self);
    fn clear_tentatives(&mut self);

    fn ports_ssp(&self) -> Vec<SSPoint>;
    fn ports_occupy_ssp(&self, ssp: SSPoint) -> bool;
    fn stroke_bounds(&self, vct: VCTransform, frame: &mut Frame, stroke: Stroke);
    fn stroke_symbol(&self, vct_composite: VCTransform, frame: &mut Frame, stroke: Stroke);
    fn bounds(&self) -> &SSBox;
    fn set_translation(&mut self, v: SSPoint);
    fn pre_translate(&mut self, ssv: Vector2D<i16, SchematicSpace>);
    fn rotate(&mut self, cw: bool);
    fn compose_transform(&self, vct: VCTransform) -> Transform2D<f32, ViewportSpace, CanvasSpace>;
}
impl <T> DeviceExt for Device<T> {
    fn get_interactable(&self) -> Interactable {
        self.interactable
    }
    fn get_transform(&self) -> Transform2D<i16, SchematicSpace, SchematicSpace> {
        self.transform
    }
    fn set_tentative(&mut self) {
        self.interactable.tentative = true;
    }
    fn draw_selected_preview(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        if self.interactable.selected {
            self.draw_selected(vct, vcscale, frame);
        }
    }
    fn tentative_by_vsb(&mut self, vsb: &VSBox) {
        if self.interactable.bounds.cast().cast_unit().intersects(vsb) {
            self.interactable.tentative = true;
        }
    }
    fn tentatives_to_selected(&mut self) {
        self.interactable.selected = self.interactable.tentative;
        self.interactable.tentative = false;
    }
    fn move_selected(&mut self, ssv: Vector2D<i16, SchematicSpace>) {
        self.pre_translate(ssv.cast_unit());
        self.interactable.selected = false;
    }
    fn clear_selected(&mut self) {
        self.interactable.selected = false;
    }
    fn clear_tentatives(&mut self) {
        self.interactable.tentative = false;
    }
    
    fn ports_ssp(&self) -> Vec<SSPoint> {
        self.graphics.ports().iter().map(|p| self.transform.transform_point(p.offset)).collect()
    }   
    fn ports_occupy_ssp(&self, ssp: SSPoint) -> bool {
        for p in self.graphics.ports() {
            if self.transform.transform_point(p.offset) == ssp {
                return true;
            }
        }
        false
    }
    fn stroke_bounds(&self, vct: VCTransform, frame: &mut Frame, stroke: Stroke) {
        self.graphics.stroke_bounds(vct, frame, stroke);
    }
    fn stroke_symbol(&self, vct: VCTransform, frame: &mut Frame, stroke: Stroke) {
        self.graphics.stroke_symbol(vct, frame, stroke);
    }
    fn bounds(&self) -> &SSBox {
        &self.interactable.bounds
    }
    fn set_translation(&mut self, v: SSPoint) {
        self.transform.m31 = v.x;
        self.transform.m32 = v.y;
        self.interactable.bounds = self.transform.outer_transformed_box(self.graphics.bounds());
    }
    fn pre_translate(&mut self, ssv: Vector2D<i16, SchematicSpace>) {
        self.transform = self.transform.pre_translate(ssv);
        self.interactable.bounds = self.transform.outer_transformed_box(self.graphics.bounds()); //self.device_type.as_ref().get_bounds().cast().cast_unit()
    }
    fn rotate(&mut self, cw: bool) {
        if cw {
            self.transform = self.transform.cast::<f32>().pre_rotate(Angle::frac_pi_2()).cast();
        } else {
            self.transform = self.transform.cast::<f32>().pre_rotate(-Angle::frac_pi_2()).cast();
        }
        self.interactable.bounds = self.transform.cast().outer_transformed_box(&self.graphics.bounds().clone().cast().cast_unit());
    }
    fn compose_transform(&self, vct: VCTransform) -> Transform2D<f32, ViewportSpace, CanvasSpace> {
        self.transform
        .cast()
        .with_destination::<ViewportSpace>()
        .with_source::<ViewportSpace>()
        .then(&vct)
    }
}
impl <T> Drawable for Device<T> {
    fn draw_persistent(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        self.graphics.draw_persistent(vct, vcscale, frame);
    }
    fn draw_selected(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        self.graphics.draw_selected(vct, vcscale, frame);
    }
    fn draw_preview(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        self.graphics.draw_preview(vct, vcscale, frame);
    }
}

struct DeviceSet <T> where T: DeviceType<T> {
    vec: Vec<Rc<RefCell<Device<T>>>>, 
    wm: usize,
    graphics_resources: Vec<Rc<Graphics<T>>>,
}
impl<T> DeviceSet<T> where T: DeviceType<T> + 'static {
    fn new_instance(&mut self) -> Rc<RefCell<Device<T>>> {
        self.wm += 1;
        let t = Rc::new(RefCell::new(Device::<T>::new_with_ord(self.wm, self.graphics_resources[0].clone())));
        self.vec.push(t.clone());
        t
    }
    fn new() -> Self {
        DeviceSet { vec: vec![], wm: 0, graphics_resources: vec![Rc::new(T::default_graphics())] }
    }
    fn devices_traits(&self) -> Vec<Rc<RefCell<dyn DeviceExt>>> {
        self.vec.iter().map(|x| x.clone() as Rc<RefCell<dyn DeviceExt>>).collect()
    }
}

pub struct Devices {
    set_r: DeviceSet<R>,
    set_gnd: DeviceSet<Gnd>,
}

impl Default for Devices {
    fn default() -> Self {
        Devices{ set_r: DeviceSet::new(), set_gnd: DeviceSet::new() }
    }
}

impl Drawable for Devices {
    fn draw_persistent(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        for d in self.iter_device_traits() {
            let vct_c = d.borrow().compose_transform(vct);
            d.borrow().draw_persistent(vct_c, vcscale, frame);
        }
    }
    fn draw_selected(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        for d in self.iter_device_traits().iter().filter(|&d| d.borrow().get_interactable().selected) {
            let vct_c = d.borrow().compose_transform(vct);
            d.borrow().draw_selected(vct_c, vcscale, frame);
        }
    }
    fn draw_preview(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        for d in self.iter_device_traits().iter().filter(|&d| d.borrow().get_interactable().tentative) {
            let vct_c = d.borrow().compose_transform(vct);
            d.borrow().draw_preview(vct_c, vcscale, frame);
        }
    }
}

impl Devices {
    pub fn place_res(&mut self) -> Rc<RefCell<dyn DeviceExt>> {
        self.set_r.new_instance()
    }
    pub fn place_gnd(&mut self) -> Rc<RefCell<dyn DeviceExt>> {
        self.set_gnd.new_instance()
    }
    pub fn iter_device_traits(&self) -> Vec<Rc<RefCell<dyn DeviceExt>>> {
        [
            self.set_gnd.devices_traits(),
            self.set_r.devices_traits(),
        ].concat()
    }
    pub fn ports_ssp(&self) -> Vec<SSPoint> {
        self.set_gnd.vec.iter().flat_map(|d| d.borrow().ports_ssp())
        .chain(self.set_r.vec.iter().flat_map(|d| d.borrow().ports_ssp()))
        .collect()
    }
    pub fn tentatives_to_selected(&mut self) {
        for d in self.iter_device_traits() {
            d.borrow_mut().tentatives_to_selected();
        }
    }
    pub fn move_selected(&mut self, ssv: Vector2D<i16, SchematicSpace>) {
        for d in self.iter_device_traits() {
            d.borrow_mut().move_selected(ssv);
        }
    }
    pub fn draw_selected_preview(&self, vct: VCTransform, vcscale: f32, frame: &mut Frame) {
        for d in self.iter_device_traits() {
            d.borrow_mut().draw_selected_preview(vct, vcscale, frame);
        }
    }
    pub fn clear_selected(&mut self) {
        for d in self.iter_device_traits() {
            d.borrow_mut().clear_selected();
        }
    }
    pub fn clear_tentatives(&mut self) {
        for d in self.iter_device_traits() {
            d.borrow_mut().clear_tentatives();
        }
    }
    pub fn bounding_box(&self) -> VSBox {
        let vt = self.iter_device_traits();
        let pts = vt.iter()
        .flat_map(
            |d| 
            [d.borrow().bounds().min, d.borrow().bounds().max].into_iter()
        );
        SSBox::from_points(pts).cast().cast_unit()
    }
    pub fn delete_selected(&mut self) {
        self.set_gnd.vec = self.set_gnd.vec.iter().filter_map(|e| {
            if !e.borrow().interactable.selected {Some(e.clone())} else {None}
        }).collect();
        self.set_r.vec = self.set_r.vec.iter().filter_map(|e| {
            if !e.borrow().interactable.selected {Some(e.clone())} else {None}
        }).collect();
    }
    pub fn occupies_ssp(&self, ssp: SSPoint) -> bool {
        for d in self.iter_device_traits() {
            if d.borrow().ports_occupy_ssp(ssp) {return true}
        }
        false
    }
}


