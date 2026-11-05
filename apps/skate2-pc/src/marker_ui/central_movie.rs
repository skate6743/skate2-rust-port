//! CentralHUD826bbf00/826bc398 and original hud2/hudcentral, fingerprintd85e4167.
use super::{assets::Assets,clips::Clip};
use crate::gpu_renderer::RenderResult;
/// `_global.ScreenManager.mColour_OffWhite`, `mColour_Red` and `mColour_Green`: the
/// `textColor` `ShowMessage` gives `eMessageColour` White, Red and Green.
const OFF_WHITE:u32=0xffff_feec;
const RED:u32=0xff97_230b;
const GREEN:u32=0xff82_be19;
pub(super) struct CentralMovie{pub root:Clip,message_visible:bool,ticks:u32,
    /// `mModMsgVisible`, `mModTicks` and Skate 3's `mIsModMessageForcedToStayUp` (TU3 +637).
    modified_visible:bool,modified_ticks:u32,modified_held:bool,
    pub sounds:Vec<&'static str>}
impl CentralMovie{
    /// CentralHUD state12 -> original SetDistrictZoneName action3adc.
    /// The authored district timeline owns its intro, hold and outro.
    pub fn district(&mut self,assets:&Assets,key:&str)->RenderResult<()> {
        let text=assets.localize(key)?.encode_utf16().collect::<Vec<_>>();
        let district=self.root.named(&["mcDistrict"])?;
        for name in ["labelText","labelTextGlow"] {
            district.named(&["mcDistrictTitle",name])?.text=Some(text.clone());
        }
        district.goto(assets,"intro")
    }
    pub fn new(assets:&Assets)->RenderResult<Self>{Ok(Self{root:Clip::new(assets,14,0)?,message_visible:false,ticks:0,
        modified_visible:false,modified_ticks:0,modified_held:false,sounds:Vec::new()})}
    /// Messages from the trick backend have no format arguments.
    pub fn show(&mut self,assets:&Assets,key:&str,hud_visible:bool)->RenderResult<()>{
        self.show_formatted(assets,key,&[],0,hud_visible)
    }
    /// CentralHUD::ShowCentralMessage 0x826bbf3c..0x826bbf60 formats the four
    /// Lua string parameters before sending screen state 4. Last message wins.
    pub fn show_formatted(&mut self,assets:&Assets,key:&str,args:&[String],colour:i32,hud_visible:bool)->RenderResult<()>{
        self.ticks=0;self.message_visible=true;
        let text=super::localizer::format_string_text(assets,key,args)?;
        if text.len()>127{return Err("native CentralHUD message length assertion".into());}
        if !hud_visible{return Ok(());}
        self.show_message(assets,"mcMessageText",text,colour)?;
        self.sounds.push("FE_SOUND_CENTRAL_MESSAGE_DEFAULT");
        Ok(())
    }
    /// CentralHUD::ShowModifiedCentralMessage 0x826bc050 (TU3 0x826640f0): the line under the
    /// central message, formatted as it is, then screen state 10, `ShowMessage` of
    /// `mcModifiedMessage` with no audio hook. A held line stays until `clear_modified`.
    pub fn show_modified(&mut self,assets:&Assets,key:&str,args:&[String],colour:i32,held:bool,hud_visible:bool)->RenderResult<()>{
        self.modified_ticks=0;self.modified_visible=true;self.modified_held=held;
        let text=super::localizer::format_string_text(assets,key,args)?;
        if text.len()>127{return Err("native CentralHUD modified message length assertion".into());}
        if !hud_visible{return Ok(());}
        self.show_message(assets,"mcModifiedMessage",text,colour)
    }
    /// Skate 3's ClearModifiedCentralMessage 0x828508e0: no longer held, then screen state 88,
    /// the line's outro.
    pub fn clear_modified(&mut self,assets:&Assets)->RenderResult<()>{
        self.modified_held=false;
        self.root.named(&["mcModifiedMessage"])?.goto(assets,"outro")
    }
    /// hudcentral `ShowMessage` 0x3682: the text in both fields, centred, `labelText` in the
    /// message colour, then the clip's intro.
    fn show_message(&mut self,assets:&Assets,clip:&str,text:Vec<u16>,colour:i32)->RenderResult<()>{
        let colour=match colour{1=>RED,2=>GREEN,_=>OFF_WHITE};
        let root=self.root.named(&[clip])?;
        for name in ["labelText","labelTextGlow"]{
            let field=root.named(&["mcText2","mcTextStringWhite",name])?;
            field.text=Some(text.clone());field.properties.insert("text_alignment".into(),"2".into());
            if name=="labelText"{field.properties.insert("text_colour".into(),colour.to_string());}
        }
        // Authored ShowMessage accesses an absent mBacking in this asset;
        // undefined clip calls have no visual effect. Do not invent a backing.
        root.goto(assets,"intro")
    }
    /// Native counters are per frontend update, independent of dT: the message's 180 then
    /// state 5, the modified line's 180 (none while held) then state 11, `HideMessage`'s outro.
    pub fn update(&mut self,assets:&Assets)->RenderResult<()>{
        if self.message_visible{self.ticks=self.ticks.wrapping_add(1);if self.ticks==180{self.force_outro(assets)?;}}
        if self.modified_visible&&!self.modified_held{
            self.modified_ticks=self.modified_ticks.wrapping_add(1);
            if self.modified_ticks==180{
                self.modified_visible=false;self.modified_ticks=0;
                self.root.named(&["mcModifiedMessage"])?.goto(assets,"outro")?;
            }
        }
        Ok(())
    }
    pub fn force_outro(&mut self,assets:&Assets)->RenderResult<()>{
        if self.message_visible{self.message_visible=false;self.ticks=0;self.root.named(&["mcMessageText"])?.goto(assets,"outro")?;}
        Ok(())
    }
    /// CentralHUD.Show(false)826c0180 ->Clear826bbe90->SetState3/HideAll.
    pub fn clear(&mut self,assets:&Assets)->RenderResult<()>{
        self.message_visible=false;self.ticks=0;
        self.modified_visible=false;self.modified_ticks=0;
        // Original HideAll invokes HideMessage twice, then HideCountdown
        // stops the root. District has its own lifetime and is untouched.
        for name in ["mcMessageText","mcModifiedMessage"]{self.root.named(&[name])?.goto(assets,"outro")?;}
        self.root.playback.stop();
        Ok(())
    }
    pub fn tick(&mut self,assets:&Assets)->RenderResult<()>{
        self.root.tick(assets)?;
        let mut calls=Vec::new();self.root.take_calls(&mut calls);
        for call in calls{match call{"CentralGenMessage"=>self.sounds.push("PlayGenCentralMsge"),
            _=>return Err(format!("untranslated CentralHUD callback {call}").into())}}
        Ok(())
    }
}
/// hudcentral.apt's sole InitAction defines the guarded global class and its
/// methods/constants, represented by CentralMovie above. It does not run the
/// constructor's stop/RegisterScreen: root action3340 instantiates that class.
/// Asset14 is fingerprint-checked (d85e4167) before any Clip is constructed.
pub(super) fn init_action(asset:usize,character:u32,offset:usize)->bool{
    matches!((asset,character,offset),(14,17,0x3390))
}
pub(super) fn action(clip:&mut Clip,offset:usize)->RenderResult<()>{
    match offset{
        0x3340=>clip.playback.stop(),
        0x3b54|0x3b94|0x3bd4|0x3c34|0x3c94|0x3ce8=>{clip.visible=false;clip.playback.stop();},
        // mcModifiedMessage's outro starts hidden.
        0x3c20=>clip.visible=false,
        0x3b70|0x3c40=>clip.visible=true,
        0x3bf4|0x3cb4=>{clip.visible=true;clip.calls.push("CentralGenMessage");},
        0x3c54=>{clip.visible=true;clip.playback.stop();},
        _=>return Err(format!("untranslated CentralHUD APT action {offset:#x}").into()),
    }Ok(())
}
