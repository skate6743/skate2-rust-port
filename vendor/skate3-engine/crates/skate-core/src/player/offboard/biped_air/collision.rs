//! Air-owned response82D2F748; never changes the shared contact/joint solver.
use super::{DT, Frame, State, Vector, math::*};
use crate::player::offboard::air_launch::Packet;
#[cfg(test)]
#[path = "collision_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug)]
pub struct Response {
    ///82D2F93C..950: byte52/value184/requestcounter200.
    pub request_52: bool,
    pub restart: Option<Vector>,
}

///82C1E170: remove only a positive component along the supplied direction.
///Its dot uses the normalized direction, subtraction uses the original one.
fn remove_positive(value: Vector, direction: Vector) -> Vector {
    let component = dot(value, normalize_or(direction, ZERO));
    if component > 0. {
        sub(value, scale(direction, component))
    } else {
        value
    }
}

impl State {
    pub fn collision_response(
        &mut self,
        errors_16288_16304: [Vector; 2],
        up_544: Vector,
        restart_allowed_8493: bool,
    ) -> Response {
        //82D2F7A8..888: the sign masks take ABSOLUTE projections and
        //absolute component products. This is not ordinary plane rejection.
        let vertical = errors_16288_16304.map(|v| dot(v, up_544).abs());
        let residual = std::array::from_fn::<_, 2, _>(|i| {
            length(sub(
                errors_16288_16304[i],
                scale(up_544, vertical[i]).map(f32::abs),
            ))
        });
        //82D2F918..938 tests BOTH projections and BOTH residual lengths.
        let request_52 = vertical.iter().chain(&residual).any(|&v| v > 0.2);
        //82D2F810/818/830/838: original permute inserts zeroY and repeatsX inW.
        let planar = errors_16288_16304.map(|v| [v[0], 0., v[2], v[0]]);
        let sizes = planar.map(length);
        let mut response = Response {
            request_52,
            restart: None,
        };
        //82098E1C,82098E28,82098E2C all contain3C23D70A in original S3.
        if !(sizes[0] > 0.01 || sizes[1] > 0.01) {
            return response;
        }
        if self.flags_544_550[6] {
            self.flags_544_550[3] = true; //82D2FA10..20
            return response;
        }
        //82D2FA24..40: strict greater; a tie chooses16304.
        let normal = normalize(if sizes[0] > sizes[1] {
            planar[0]
        } else {
            planar[1]
        });
        let velocity = self.result.velocity_288;
        if (sizes[0] > 0.01 || sizes[1] > 0.01) && dot(normal, velocity) < -10. {
            self.flags_544_550[3] = true; //82D2FA70..FB44
        }
        if !(dot(velocity, normal) < 0.) {
            return response;
        } //82D2FB4C..5C
        let above_root =
            self.result.scalar_392 > self.frame_208[3][1] && self.result.normal_304[1] > 0.7; //82D2FB64..BF0
        if restart_allowed_8493 && !above_root {
            response.restart = Some(normal); //82D2FBF8..FC14
        }
        response
    }

    ///82D2FC18..FD58: caller first produces the complete native launch packet.
    ///Secondary velocity and all other producer fields retain their own values.
    pub fn restart_packet(
        &mut self,
        mut packet: Packet,
        normal: Vector,
        position_592: Vector,
        up_544: Vector,
        foot_height: f32,
    ) -> Packet {
        self.flags_544_550[2] = true;
        //The source computes two normalized directions; the second explicitly
        //zerosY. Both are planar here becauseF810..838 already removedY.
        let horizontal = normal;
        let first = remove_positive(self.result.velocity_288, scale(horizontal, -1.));
        let second = remove_positive(first, scale(normal, -1.));
        packet.velocity_0 = madd(normal, 1., second); //82D2FCC4
        packet.board_position_80 = madd(up_544, foot_height, position_592); //FD44
        packet.position_32 = madd(packet.velocity_0, DT, position_592); //FD48
        packet.has_board_position_116 = true; //FD30
        self.restart_normal_528 = normal; //FD28
        packet
    }

    ///Only after the host successfully submits the restart82D6CA58.
    pub fn finish_restart(&mut self, effective_animation_frame: Frame, normal: Vector) {
        self.frame_452 = 0; //82D2FD5C
        self.flags_544_550[0] = false;
        self.flags_544_550[1] = false;
        self.frame_80 = effective_animation_frame; //82D2FD70..DA4
        self.initial_up_480 = normal; //82D2FDA8, NOT frame80's up
    }

    ///82D2F490..534 adjusts the current sample after a restart; it does NOT
    ///resample or overwrite the persistent selector's new trajectory.
    pub fn correct_restarted_sample(&mut self) {
        if self.flags_544_550[2] {
            let step = scale(self.result.velocity_288, DT);
            self.result.position_272 = add(
                sub(self.result.position_272, step),
                remove_positive(step, scale(self.restart_normal_528, -1.)),
            );
        }
    }
}
