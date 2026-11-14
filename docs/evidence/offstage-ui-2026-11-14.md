# UI parked off the stage in other window shapes — November 14, 2026

PC port addition: window shapes the original never had.

The original draws its UI on one 1280x720 target, the screen. Its APT movies keep the pieces not
in use at that moment just off the stage, below, above or beside it, where the screen's edge hides
them. The port keeps the stage whole and centred in a window of any shape and widens the world
around it (`window_clip_from_world`, `live_skating_view.rs`): a window narrower than 16:9 has room
above and below the stage, a wider one beside it, and the parked pieces showed there.

## The pieces

Found with `SKATE2_TRACE_OFFSTAGE`, which logs each draw left out once, with its bounds on the
1280x720 stage. The runs used 800x1800 and 2560x1080 windows and covered the career HUD,
tricks, the replay editor, the main menu, Extras, the pause hub and map, Edit Skater, Free Skate
and Options:

| Movie | Pieces | Stage bounds |
|---|---|---|
| actionoverlay2 (SignUpHUD) | `mSignup`: its label and glow, the sign-up symbol, its button | y 814..905 (below) |
| trickdisplay2 (the trick score) | `multiplier_mc`, `multiTimer_mc`: the multiplier ring's pieces, a grey-white disc under the score | y 751..879 (below), x -368..-84 (left) |
| chyron | `mcChyron/testlength`, the text field the song chyron measures titles with | y 772..788 (below) |
| replay_editor2 | `mcDropMarker`, `mcRemoveMarker`, `mcPrevMarker`, `mcNextMarker`, `mcCamRotate`, `mcCamLR`, `mcCamUpDown`, `mcCamZoom`, `mcLBMove`, `mcRBMove` | y -82..-8 (above) |
| replay_editor2 | `mEffects` (the Pro Edit Menu: the cameras, Speed, Shake, Watermark and its buttons), `mHide`, `mHideText` | y 744..1392 (below) |

The pause hub, the map, Edit Skater, Free Skate, Options, the main menu and Extras park nothing.

## The rule

`offstage` (`marker_ui.rs`) applies to one UI draw at a time: a shape fill, a text field or a
custom component's draw. When every vertex lies beyond the same edge of the stage's place in the
window, the draw is left out. A draw the stage shows any part of is drawn as before, including its
parts beyond the stage. In a 16:9 window the rule never applies.

Screens drawn flat and then projected are tested where the projection puts them. These are the 3D
HUD placements (`project_focus_right`) and the spin container (`project_spin`).

Kept whole is whatever the port itself carries to the window's edges. That means the stretched
backgrounds and dimmers, and the letterbox's bars, film strips, title and buttons
(`letterbox_edge`); in a wider window the film strips run on along the stretched bars. Player names,
chase arrows and the screen fades are drawn apart from this UI and are unchanged.

## Verified

Release build, scripted runs on scratch saves, with the previous build run the same way:

- 800x1800, career: the sign-up prompt under the stage is gone. The minimap and the rest are
  unchanged.
- 800x1800, tricks: the disc under the "Kickflip 20" score is gone. The score and its multiplier
  ring in use are unchanged.
- 800x1800, the replay editor: the button row above the stage and the Pro Edit Menu below it are
  gone. The timeline, its buttons and the status bar are unchanged.
- 800x1800, the main menu and Extras: unchanged, with the letterbox at the window's top and
  bottom.
- 2560x1080, the main menu and Extras: the letterbox's bars and film strips are unchanged across
  the whole width.
- 2560x1080, tricks: the score is unchanged.
- 1920x1080, career and tricks: no draw left out.
