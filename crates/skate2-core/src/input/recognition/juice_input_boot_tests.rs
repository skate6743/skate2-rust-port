use super::*;
use crate::input_cvar_runtime::InputCvarManager;
struct Platform {
    calls: Vec<&'static str>,
    initialized: bool,
    connected: bool,
    ip: [u8; 4],
    name: Option<Vec<u8>>,
    endpoint: Option<JuiceEndpoint>,
    observer: Option<JuiceInputBoot<u32>>,
}

#[cfg(feature = "skate3-runtime")]
#[path = "../../session/skate3_player_corpus.rs"]
mod skate3_player_corpus;
#[path = "../../session/selected_session_cross_owner_frame_corpus.rs"]
mod selected_session_cross_owner_frame_corpus;
#[path = "../../session/selected_session_input_identity_corpus.rs"]
mod selected_session_input_identity_corpus;
#[path = "../../session/selected_session_public_facade_corpus.rs"]
mod selected_session_public_facade_corpus;
impl Platform {
    fn new() -> Self {
        Self {
            calls: vec![],
            initialized: true,
            connected: false,
            ip: [192, 168, 1, 2],
            name: None,
            endpoint: None,
            observer: None,
        }
    }
    fn observe_ready_without_reentry_lock(&self) {
        if let Some(boot) = &self.observer {
            assert!(
                !boot
                    .state
                    .try_lock()
                    .expect("platform callbacks must not hold the owner lock")
                    .ready
            );
        }
    }
}
impl JuiceBootPlatform for Platform {
    type Error = &'static str;
    type Connection = u32;
    fn initialize_network(&mut self) -> Result<bool, Self::Error> {
        self.observe_ready_without_reentry_lock();
        self.calls.push("initialize");
        Ok(self.initialized)
    }
    fn debug_address(&mut self) -> Result<[u8; 4], Self::Error> {
        self.observe_ready_without_reentry_lock();
        self.calls.push("debug_address");
        Ok(self.ip)
    }
    fn console_name(&mut self) -> Result<JuiceConsoleName, Self::Error> {
        self.observe_ready_without_reentry_lock();
        self.calls.push("console_name");
        Ok(JuiceConsoleName {
            status: if self.name.is_some() { 47_841_280 } else { 0 },
            bytes: self.name.clone(),
        })
    }
    fn connect(&mut self, endpoint: &JuiceEndpoint) -> Result<JuiceConnection<u32>, Self::Error> {
        self.observe_ready_without_reentry_lock();
        self.calls.push("connect");
        self.endpoint = Some(endpoint.clone());
        Ok(if self.connected {
            JuiceConnection::Connected(17)
        } else {
            JuiceConnection::Failed
        })
    }
    fn sleep_milliseconds(&mut self, milliseconds: u32) -> Result<(), Self::Error> {
        self.observe_ready_without_reentry_lock();
        assert_eq!(milliseconds, 2000);
        self.calls.push("sleep2000");
        Ok(())
    }
}
fn cvars() -> (InputCvarManager, InputCvarReferences) {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    (manager, refs)
}
const CONFIG:&[u8]=b"[JUICECONFIG]\r\nenable=1\r\nserverip=10.10.226.22\r\nserverip_HTC=10.107.84.18\r\nserverip_Madrid=10.20.129.141\r\nserverip_VMC=192.168.80.13\r\nserverport=31001\r\n";
#[test]
fn explicit_disabled_override_initializes_network_but_never_samples_endpoint() {
    let juice = JuiceTrickInput::new_static_initialized();
    let boot = JuiceInputBoot::new_static_initialized(juice.clone());
    let mut platform = Platform::new();
    platform.initialized = false;
    boot.try_construct(
        Some(CONFIG),
        JuiceHostOption::DisableBeforeStart,
        &mut platform,
    )
    .unwrap();
    assert!(boot.source_enabled().unwrap());
    assert!(Arc::ptr_eq(&juice.state, &boot.trick_input().state));
    let (_manager, refs) = cvars();
    boot.try_start(&refs, &mut platform).unwrap();
    assert_eq!(platform.calls, ["initialize"]);
    let state = boot.state.lock().unwrap();
    assert_eq!(state.phase, JuiceBootPhase::Disabled);
    assert_eq!(state.option, JuiceHostOption::DisableBeforeStart);
    assert_eq!(state.network_initialized, Some(false));
    assert!(!state.ready);
}
#[test]
fn original_enabled_failure_attempts_once_and_retains_selected_endpoint() {
    let boot = JuiceInputBoot::new_static_initialized(JuiceTrickInput::new_static_initialized());
    let mut platform = Platform::new();
    platform.observer = Some(boot.clone());
    platform.ip = [10, 107, 4, 2];
    platform.name = Some(b"VMC-console".to_vec());
    boot.try_construct(
        Some(CONFIG),
        JuiceHostOption::OriginalConfiguration,
        &mut platform,
    )
    .unwrap();
    let (mut manager, refs) = cvars();
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkEnableJuiceConnectionRetry=1\r\n"))
        .unwrap();
    boot.try_start(&refs, &mut platform).unwrap();
    assert_eq!(
        platform.calls,
        [
            "initialize",
            "debug_address",
            "console_name",
            "connect",
            "sleep2000"
        ]
    );
    assert_eq!(
        platform.endpoint,
        Some(JuiceEndpoint {
            address: b"192.168.80.13".to_vec(),
            port: 31001
        })
    );
    assert_eq!(
        boot.state.lock().unwrap().phase,
        JuiceBootPhase::ConnectionFailed
    );
    assert_eq!(boot.state.lock().unwrap().wait_for_ip, Some(true));
    assert!(matches!(
        boot.try_start(&refs, &mut platform),
        Err(JuiceBootError::Phase(JuiceBootPhase::ConnectionFailed))
    ));
}

#[test]
fn malformed_late_ini_retains_prior_field_without_claiming_completed_configuration() {
    let boot = JuiceInputBoot::new_static_initialized(JuiceTrickInput::new_static_initialized());
    let mut platform = Platform::new();
    assert!(matches!(
        boot.try_construct(
            Some(b"[JUICECONFIG]\nenable=1\nserverip=123456789012345678901234567890123\n"),
            JuiceHostOption::OriginalConfiguration,
            &mut platform
        ),
        Err(JuiceBootError::Ini(JuiceIniError::UnterminatedAddress))
    ));
    assert!(boot.state.lock().unwrap().config.enabled);
    assert_eq!(boot.state.lock().unwrap().phase, JuiceBootPhase::Config);
    assert!(boot.source_enabled().is_err());
    assert!(platform.calls.is_empty());
}
#[test]
fn failed_network_initialization_still_selects_endpoint_and_sleeps_without_connect() {
    let boot = JuiceInputBoot::new_static_initialized(JuiceTrickInput::new_static_initialized());
    let mut platform = Platform::new();
    platform.initialized = false;
    boot.try_construct(
        Some(CONFIG),
        JuiceHostOption::OriginalConfiguration,
        &mut platform,
    )
    .unwrap();
    let (_manager, refs) = cvars();
    boot.try_start(&refs, &mut platform).unwrap();
    assert_eq!(
        platform.calls,
        ["initialize", "debug_address", "console_name", "sleep2000"]
    );
}
#[test]
fn successful_socket_is_retained_without_installation_or_readiness() {
    let boot = JuiceInputBoot::new_static_initialized(JuiceTrickInput::new_static_initialized());
    let mut platform = Platform::new();
    platform.connected = true;
    boot.try_construct(
        Some(CONFIG),
        JuiceHostOption::OriginalConfiguration,
        &mut platform,
    )
    .unwrap();
    let (_manager, refs) = cvars();
    assert!(matches!(
        boot.try_start(&refs, &mut platform),
        Err(JuiceBootError::HandshakePending)
    ));
    assert_eq!(boot.state.lock().unwrap().connection, Some(17));
    assert!(!boot.state.lock().unwrap().ready);
    let mut devices = InputDeviceManager::new();
    devices.initialize().unwrap();
    assert!(matches!(
        boot.try_install(&mut devices),
        Err(JuiceBootError::Phase(JuiceBootPhase::HandshakePending))
    ));
}
#[test]
fn installed_same_owner_remap_preserves_all_lane_bits_and_rejects_foreign_handle() {
    let mut boot =
        JuiceInputBoot::new_static_initialized(JuiceTrickInput::new_static_initialized());
    let mut platform = Platform::new();
    boot.try_construct(None, JuiceHostOption::OriginalConfiguration, &mut platform)
        .unwrap();
    let (_manager, refs) = cvars();
    boot.try_start(&refs, &mut platform).unwrap();
    let mut devices = InputDeviceManager::new();
    devices.initialize().unwrap();
    boot.try_install(&mut devices).unwrap();
    let handle = boot.try_remap_handle().unwrap();
    let before = std::array::from_fn(|i| f32::from_bits(0x8000_0000 | u32::try_from(i).unwrap()));
    let mut values = before;
    boot.update(&handle, 0, &mut values).unwrap();
    assert_eq!(values.map(f32::to_bits), before.map(f32::to_bits));
    assert!(matches!(
        boot.update(&JuiceRemapHandle(Arc::new(())), 0, &mut values),
        Err(JuiceBootError::ForeignRemap)
    ));
}
#[test]
fn ini_applies_case_insensitive_last_duplicate_and_native_quote_comment_rules() {
    let mut config = JuiceConfig::default();
    ini::apply(&mut config,b"\xef\xbb\xbf[ juiceconfig ]\nenable=1\nENABLE = 0\nserverip = \" 1.2.3.4\" //done\nserverport=port31001\n").unwrap();
    assert!(!config.enabled);
    assert_eq!(config.server.as_deref(), Some(b" 1.2.3.4 ".as_slice()));
    assert_eq!(config.port, 31001);
}

#[test]
fn malformed_cross_field_cursor_never_becomes_a_clean_section_or_key() {
    for bytes in [
        b"[JUICECONFIG\nenable=1\n".as_slice(),
        b"bad\n[JUICECONFIG]\nenable=1\n".as_slice(),
        b"bad//comment\n[JUICECONFIG]\nenable=1\n".as_slice(),
        b"[JUICE//comment\nenable=1\n".as_slice(),
    ] {
        let mut config = JuiceConfig::default();
        assert_eq!(
            ini::apply(&mut config, bytes),
            Err(JuiceIniError::UnterminatedKey)
        );
        assert!(!config.enabled);
    }
}
