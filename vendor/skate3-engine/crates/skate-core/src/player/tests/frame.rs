use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Recorder {
    calls: Rc<RefCell<Vec<FrameStage>>>,
    fail: Option<FrameStage>,
}

impl Recorder {
    fn call(&mut self, stage: FrameStage) -> Result<(), FrameStage> {
        self.calls.borrow_mut().push(stage);
        if self.fail == Some(stage) {
            Err(stage)
        } else {
            Ok(())
        }
    }
}

impl AnimationPhases for Recorder {
    type Error = FrameStage;
    fn phase_one(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::AnimationPhaseOne)
    }
    fn phase_two(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::AnimationPhaseTwo)
    }
    fn phase_three(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::AnimationPhaseThree)
    }
    fn publish_physics_input(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::ActorPhysicsPublication)
    }
}

impl WorldPhases for Recorder {
    type Error = FrameStage;
    fn start_frame(&mut self, _: f32) -> Result<(), Self::Error> {
        self.call(FrameStage::StartFrame)
    }
    fn pre_frame(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::WorldPreFrame)
    }
    fn prepare(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::WorldPrepare)
    }
    fn precursor(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::WorldPrecursor)
    }
    fn after_input(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::WorldAfterInput)
    }
    fn collect_solver_inputs(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::CollectSolverInputs)
    }
    fn finish_outputs(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::FinishOutputs)
    }
}

impl PhysicalPhases for Recorder {
    type Error = FrameStage;
    fn post_zero_jobs(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::PostZeroJobs)
    }
    fn input(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::Input)
    }
    fn post_input(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::PostInput)
    }
    fn logic(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::Logic)
    }
    fn pre_state(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::PreState)
    }
    fn state(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::State)
    }
    fn post_state(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::PostState)
    }
    fn adjust(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::Adjust)
    }
    fn post_adjust(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::PostAdjust)
    }
    fn output(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::Output)
    }
    fn post_output(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::PostOutput)
    }
}

impl SolverPhases for Recorder {
    type Error = FrameStage;
    fn prepare_jobs(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::PrepareSolverJobs)
    }
    fn submit(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::SubmitSolver)
    }
    fn wait(&mut self) -> Result<(), Self::Error> {
        self.call(FrameStage::WaitSolver)
    }
}

fn recorders(fail: Option<FrameStage>) -> (Rc<RefCell<Vec<FrameStage>>>, [Recorder; 4]) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorder = Recorder {
        calls: calls.clone(),
        fail,
    };
    (calls, core::array::from_fn(|_| recorder.clone()))
}

#[test]
fn runs_the_complete_native_order_across_the_solver_fence() {
    let (calls, [mut animation, mut world, mut physical, mut solver]) = recorders(None);
    let mut scheduler = FrameScheduler::default();
    scheduler
        .run_before_solver(
            1.0 / 60.0,
            &mut animation,
            &mut world,
            &mut physical,
            &mut solver,
        )
        .unwrap();
    assert_eq!(scheduler.state(), SchedulerState::SolverPending);
    scheduler
        .run_after_solver(&mut world, &mut physical, &mut solver)
        .unwrap();
    assert_eq!(scheduler.state(), SchedulerState::Ready);
    assert_eq!(
        *calls.borrow(),
        [
            FrameStage::StartFrame,
            FrameStage::WorldPreFrame,
            FrameStage::AnimationPhaseOne,
            FrameStage::AnimationPhaseTwo,
            FrameStage::WorldPrepare,
            FrameStage::AnimationPhaseThree,
            FrameStage::WorldPrecursor,
            FrameStage::ActorPhysicsPublication,
            FrameStage::PostZeroJobs,
            FrameStage::Input,
            FrameStage::WorldAfterInput,
            FrameStage::PostInput,
            FrameStage::Logic,
            FrameStage::PreState,
            FrameStage::State,
            FrameStage::PostState,
            FrameStage::CollectSolverInputs,
            FrameStage::PrepareSolverJobs,
            FrameStage::SubmitSolver,
            FrameStage::WaitSolver,
            FrameStage::Adjust,
            FrameStage::PostAdjust,
            FrameStage::Output,
            FrameStage::PostOutput,
            FrameStage::FinishOutputs,
        ]
    );
}

#[test]
fn post_solver_work_cannot_run_before_submission() {
    let (_, [_, mut world, mut physical, mut solver]) = recorders(None);
    let mut scheduler = FrameScheduler::default();
    assert_eq!(
        scheduler.run_after_solver(&mut world, &mut physical, &mut solver),
        Err(ScheduleError::WrongState {
            expected: SchedulerState::SolverPending,
            actual: SchedulerState::Ready,
        })
    );
}

#[test]
fn a_failed_phase_stops_the_frame_and_cannot_publish_later_output() {
    let (calls, [mut animation, mut world, mut physical, mut solver]) =
        recorders(Some(FrameStage::Logic));
    let mut scheduler = FrameScheduler::default();
    assert_eq!(
        scheduler.run_before_solver(
            1.0 / 60.0,
            &mut animation,
            &mut world,
            &mut physical,
            &mut solver
        ),
        Err(ScheduleError::Runtime {
            stage: FrameStage::Logic,
            source: FrameStage::Logic
        })
    );
    assert_eq!(scheduler.state(), SchedulerState::Faulted);
    assert_eq!(calls.borrow().last(), Some(&FrameStage::Logic));
    assert!(!calls.borrow().contains(&FrameStage::SubmitSolver));
}
