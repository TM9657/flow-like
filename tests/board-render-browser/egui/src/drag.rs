#[derive(Clone, Copy, Debug)]
pub enum PointerEvent {
    Press { position: [f32; 2], button: u8 },
    Move([f32; 2]),
    Release { position: [f32; 2], button: u8 },
}

#[derive(Default, Debug)]
pub struct PointerFrame {
    pub press: Option<([f32; 2], u8)>,
    pub release: Option<[f32; 2]>,
    pub latest: Option<[f32; 2]>,
}

pub fn pointer_frame(
    events: impl Iterator<Item = PointerEvent>,
    active: Option<u8>,
) -> PointerFrame {
    let mut result = PointerFrame::default();
    let mut captured = active;
    for event in events {
        match event {
            PointerEvent::Press { position, button } if captured.is_none() => {
                captured = Some(button);
                result.press = Some((position, button));
                result.latest = Some(position);
            }
            PointerEvent::Move(position) => result.latest = Some(position),
            PointerEvent::Release { position, button } if captured == Some(button) => {
                result.release = Some(position);
                result.latest = Some(position);
            }
            _ => {}
        }
    }
    result
}

#[derive(Clone, Copy)]
pub struct DragAnchor {
    pub press: [f32; 2],
    pub start: [f32; 2],
    pub scale: f32,
}

impl DragAnchor {
    pub fn position_at(self, pointer: [f32; 2]) -> [f32; 2] {
        std::array::from_fn(|axis| {
            self.start[axis] + (pointer[axis] - self.press[axis]) / self.scale
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesced_press_moves_and_release_keep_actual_endpoints() {
        let frame = pointer_frame(
            [
                PointerEvent::Press {
                    position: [20.0, 30.0],
                    button: 0,
                },
                PointerEvent::Move([40.0, 50.0]),
                PointerEvent::Move([80.0, 70.0]),
                PointerEvent::Release {
                    position: [100.0, 90.0],
                    button: 0,
                },
                PointerEvent::Move([200.0, 200.0]),
            ]
            .into_iter(),
            None,
        );
        assert_eq!(frame.press, Some(([20.0, 30.0], 0)));
        assert_eq!(frame.release, Some([100.0, 90.0]));
        let anchor = DragAnchor {
            press: frame.press.unwrap().0,
            start: [500.0, 800.0],
            scale: 0.5,
        };
        assert_eq!(anchor.position_at(frame.release.unwrap()), [660.0, 920.0]);
    }

    #[test]
    fn final_position_is_independent_of_frame_batching() {
        let anchor = DragAnchor {
            press: [20.0, 30.0],
            start: [500.0, 800.0],
            scale: 0.8,
        };
        let mut paced = anchor.start;
        for pointer in [[21.0, 31.0], [40.0, 50.0], [84.0, 78.0]] {
            paced = anchor.position_at(pointer);
        }
        assert_eq!(paced, [580.0, 860.0]);
        assert_eq!(paced, anchor.position_at([84.0, 78.0]));
    }

    #[test]
    fn release_in_a_later_frame_includes_its_final_motion() {
        let frame = pointer_frame(
            [
                PointerEvent::Move([70.0, 60.0]),
                PointerEvent::Release {
                    position: [80.0, 80.0],
                    button: 0,
                },
            ]
            .into_iter(),
            Some(0),
        );
        assert_eq!(frame.press, None);
        assert_eq!(frame.release, Some([80.0, 80.0]));
        let anchor = DragAnchor {
            press: [20.0, 30.0],
            start: [0.0, 0.0],
            scale: 1.0,
        };
        assert_eq!(anchor.position_at(frame.release.unwrap()), [60.0, 50.0]);
    }

    #[test]
    fn an_unrelated_button_release_does_not_finish_the_drag() {
        let frame = pointer_frame(
            [PointerEvent::Release {
                position: [80.0, 80.0],
                button: 1,
            }]
            .into_iter(),
            Some(0),
        );
        assert_eq!(frame.release, None);
    }
}
