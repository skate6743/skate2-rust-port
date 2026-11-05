//! Ground state56's hand manager, distinct from SkateboardController448.
use super::Vector;
#[derive(Clone, Copy, Debug)]
pub struct Hand {
    pub vectors_0_to_64: [Vector; 5],
    pub word_80: u32,
    pub flag_84: bool,
    pub scalars_96_100: [f32; 2],
    pub flags_104_to_107: [bool; 4],
}
impl Default for Hand {
    fn default() -> Self {
        Self {
            vectors_0_to_64: [[0.; 4], [0.; 4], [0., 1., 0., 0.], [0.; 4], [0.; 4]],
            word_80: 0,
            flag_84: false,
            scalars_96_100: [0.; 2],
            flags_104_to_107: [false; 4],
        }
    }
}
#[derive(Clone, Debug)]
pub struct State {
    pub hands: [Hand; 2],
    pub vectors_224_to_272: [Vector; 4],
    pub scalars_288_to_296: [f32; 3],
    pub word_300: u32,
    pub flags_304_to_307: [bool; 4],
    pub words_308_to_316: [u32; 3],
}
impl Default for State {
    fn default() -> Self {
        Self {
            hands: [Hand::default(); 2],
            vectors_224_to_272: [[0.; 4]; 4],
            scalars_288_to_296: [0.; 3],
            word_300: 2,
            flags_304_to_307: [false; 4],
            words_308_to_316: [0; 3],
        }
    }
}
/// Actual skeleton16432 object's reset fields; all other fields survive.
pub struct SkeletonReset<'a> {
    pub enabled_464: &'a mut bool,
    pub values_468: &'a mut [f32; 4],
    pub values_484: &'a mut [f32; 4],
    pub words_500: &'a mut [u32; 4],
}
impl State {
    ///82D77168 writes only the represented fields, not pointer fields320 onward
    ///or unwritten gaps88..95 in each hand record.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    ///82D770D8 preserves hand state across500/501/502 and always publishes304.
    pub fn enter(&mut self, previous_2504: u32, current_2508: u32, skeleton: SkeletonReset<'_>) {
        if !matches!(previous_2504, 500 | 501 | 502) {
            self.reset();
            *skeleton.values_468 = [0.; 4];
            *skeleton.values_484 = [0.; 4];
            *skeleton.words_500 = [0; 4];
            *skeleton.enabled_464 = false;
        }
        self.flags_304_to_307[0] = current_2508 == 502;
    }
}
