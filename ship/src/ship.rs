use jandering_engine::{
    object::{Instance, Object},
    renderer::Renderer,
    types::{Mat3, Mat4, Qua, Vec3},
    window::{Events, MouseButton},
};

use crate::ocean::Ocean;

pub struct Ship {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,

    pub direction: Vec3,

    pub in_water: bool,

    pub time: f32,
    pub mesh: Object<Instance>,
    pub is_left_mouse_held: bool,
}

impl Ship {
    pub fn new(renderer: &mut Renderer) -> Self {
        // let instances = (-50..50)
        //     .flat_map(|x| {
        //         (-50..50)
        //             .map(|y| {
        //                 Instance::default()
        //                     .translate(Vec3::new(x as f32, 0.0, y as f32))
        //                     .scale(0.2)
        //             })
        //             .collect::<Vec<_>>()
        //     })
        //     .collect();
        let mesh = Object::from_obj(
            // include_str!("cube.obj"),
            include_str!("ship.obj"),
            renderer,
            // instances,
            vec![Instance::default()],
            // vec![Instance::default(), Instance::default().scale(0.5)],
        );

        Self {
            position: Vec3::new(0.0, 10.0, 0.0),
            acceleration: Vec3::ZERO,
            velocity: Vec3::ZERO,

            direction: Vec3::NEG_Z,

            time: 0.0,
            is_left_mouse_held: false,
            in_water: false,

            mesh,
        }
    }

    pub fn update(
        &mut self,
        ocean: &Ocean,
        mouse_world: Vec3,
        events: &Events,
        renderer: &mut Renderer,
        dt: f32,
    ) {
        self.is_left_mouse_held = if events.is_mouse_pressed(MouseButton::Left) {
            true
        } else if events.is_mouse_released(MouseButton::Left) {
            false
        } else {
            self.is_left_mouse_held
        };

        self.time += dt;

        let center_height = ocean.position_at(self.position).y - 0.5;

        let was_in_water = self.in_water;
        self.in_water = self.position.y <= center_height;

        // if entering water we should lose a bunch of speed
        if !was_in_water && self.in_water {
            self.velocity.y /= 2.0;
        }

        // apply gravity
        self.acceleration.y += -9.8;

        let water_normal = ocean.normal_at(self.position);

        if self.in_water {
            self.acceleration.y += 30.0;

            if self.is_left_mouse_held {
                // let d2_direction = (Vec3::new(mouse_world.x, 0.0, mouse_world.z)
                //     - Vec3::new(self.position.x, 0.0, self.position.z))
                let d2_direction = Vec3::new(mouse_world.x, 0.0, mouse_world.z).normalize();
                self.acceleration += d2_direction * 15.0;
            }

            let downhill_direction = Vec3::new(
                water_normal.x * water_normal.y,
                -(water_normal.x * water_normal.x) - (water_normal.z * water_normal.z),
                water_normal.z * water_normal.y,
            )
            .normalize();

            let d = self.velocity.normalize().dot(downhill_direction);
            if d < 0.0 {
                self.acceleration += self.velocity * 2.0 * d;
            }
        }

        self.acceleration += -self.velocity * 0.1;

        self.velocity += self.acceleration * dt.clamp(0.0, 1.0);

        self.position += self.velocity * dt.clamp(0.0, 1.0);

        self.acceleration = Vec3::ZERO;

        if self.mesh.instances[0].mat().is_nan() {
            println!("ship mat was nan");
            self.mesh.instances[0].set_mat(Mat4::IDENTITY);
        }
        // if self.mesh.instances[1].mat().is_nan() {
        //     println!("ship mat was nan");
        //     self.mesh.instances[1].set_mat(Mat4::IDENTITY);
        // }

        self.mesh.instances[0].set_position(Vec3::new(0.0, self.position.y, 0.0));

        self.direction += (self.velocity - self.direction) * (1.0 - (-2.0 * dt).exp());
        // self.direction.x += (self.velocity.x - self.direction.x) * (1.0 - (-2.0 * dt).exp());
        // self.direction.z += (self.velocity.z - self.direction.z) * (1.0 - (-2.0 * dt).exp());
        // self.direction.y += (self.velocity.y - self.direction.y) * (1.0 - (-1.0 * dt).exp());

        let (front_position, back_position) = {
            let pos_offset = Vec3::new(self.direction.x, 0.0, self.direction.z).normalize();
            let front_offset = ocean.position_at(self.position + pos_offset);
            let back_offset = ocean.position_at(self.position - pos_offset);
            (
                self.position + pos_offset + front_offset,
                self.position - pos_offset + back_offset,
            )
        };
        let dir = -(front_position - back_position).normalize();
        // let dir = -self.direction.normalize();
        if !dir.is_nan() {
            // self.mesh.instances[1].set_position(self.position + dir * 2.0);
            let right = Vec3::Y.cross(dir).normalize();
            let recalculated_up = dir.cross(right);
            let direction_rotation = Qua::from_mat3(&Mat3::from_cols(right, recalculated_up, dir));

            let up_rotation =
                Qua::from_mat3(&Mat3::from_cols(Vec3::NEG_X, water_normal, Vec3::NEG_Z));

            if !direction_rotation.is_nan() && !up_rotation.is_nan() {
                // self.mesh.instances[0]
                //     .set_rotation_qua((up_rotation * direction_rotation).normalize());
                self.mesh.instances[0].set_rotation_qua(direction_rotation.normalize());
            }
        }

        self.mesh.update(renderer);
    }
}
