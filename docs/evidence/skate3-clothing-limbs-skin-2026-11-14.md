# Skate 3 clothing: arms, legs, skin and helper bones (2026-11-14)

This applies with a linked Skate 3 data folder (`crate::skate3`), whose clothing joins Slappy's
(`Skate3Data::clothing_for`).

## Arms and legs

- `CASAssembler::Correct` (`assemble_parts`) gives a torso the arm part it requires and pants the
  leg part they require. Each takes the first part of its slot that fits.
  - `GetArmPartForTorso` matches the torso's gender, `ArmType` equal to its `ArmRequired`, the skin
    tone and the skin normal.
  - `GetLegPartForPants` matches the gender, `LegType` equal to `LegRequired`, the skin tone and the
    socks' style.
- Skate 3's arms and legs join after Skate 2's own in their slots (`MERCHANDISE_SLOTS`, those whose
  model and material Skate 2 lacks).
  - Skate 3 remade them: the same model names (`arms_female_bicept`, `arms_female_wrist`,
    `male_arms_forearmdown` and others) on assets of its own.
  - It adds kinds Skate 2 has not: female `forearm`, `shoulder`, `shoulder with tummy`, `legs_tight`
    and `mid_thigh_down`; male `knee`, `thigh` and `legs_tight`.
  - Tones (`light`, `dark`) and normals (female `average`, `chubby`, `thin`; male `average`,
    `thickchubby`, `thickmuscle`, `thin`, `thinmuscle`) carry Skate 2's names.
- Skate 3's tops require `bicept`, `wrist`, `forearm`, `shoulder` or `shoulder with tummy`.
  Its bottoms require `knee down`, `mid_thigh_down`, `thigh down` or `legs_tight` (female), and
  `knee`, `thigh` or `legs_tight` (male).
- Where Skate 2 has the kind too (female `bicept`, `wrist`, `thigh down`), the first part that fit
  was Skate 2's, which does not meet Skate 3's sleeves and hems.
- Port: a garment that is one of Skate 3's (`CacPartDb::is_skate3_part`) looks for its arm or leg
  among Skate 3's joined parts first (`limb_for`), then among Skate 2's. Skate 2's garments keep
  the first part that fits.
  - A skater saved before wears Skate 3's arms or legs once Edit Skater closes: closing corrects the
    record's parts and composes its recipe again (`gameplay_record`).
  - Seen in Edit Skater: a female skater in Skate 3's Trefoil Tee wears Skate 3's `bicept` arm.
- `SKATE2_DIAGNOSTIC_CAS_LIMBS` logs every arm and leg part and the kinds the tops and bottoms
  require, each with the database it came from, and the tops that take the necklace off.
- `SKATE2_DIAGNOSTIC_SKATE2_LIMBS` gives every garment the first arm or leg that fits, Skate 2's
  first, for comparison.

## Necklace

- Skate 3's `CASAssembler::Correct` (0x8253f648) clears the jewellery slot (`Jewellery`, 25) after
  `CorrectTorso` when the inner or outer torso has `IsNecklaceRemoved` set (0x82540430).
  - Skate 3's part record carries the flag after `IsLogoable` (byte 106). Its runtime part holds a
    pointer per field: the slot at +0x20 (`GetInGamePart` 0x82ddbad0 reads it), then the nine
    flags from +0x24 to +0x44, `IsNecklaceRemoved` at +0x38 and `IsSkinned` at +0x3c
    (`CorrectTorso` 0x82540828 reads it for the alternate skin colour).
  - Skate 2's record has no such flag.
- Port: `CacPart::necklace_removed` (0 for Skate 2's parts), and `assemble_parts` clears slot 25
  under such a torso. The record keeps the necklace; it shows again under another top.
  - Of Skate 3's tops, 64 outer torso models carry it (jackets, hoodies, button-up shirts,
    sweaters, jerseys); no tee, and no inner torso.
  - Seen in Edit Skater: a male skater's Hall of Meat pendant is gone under Skate 3's
    `sweater_pullover_male`, and shows over Skate 2's and over Skate 3's Trefoil Tee.
  - `SKATE2_DIAGNOSTIC_EDIT_SKATER_FIRST=<title>[,<title>...]|<part>` puts a first part on (buying
    it at the cart on the way out) before the path's.

## Skin's shine

- `cacstamp_skin`'s program takes its specular mask from the diffuse map's alpha, squared.
- Skate 2's arm and leg skin diffuse maps are DXT5 with a mask: alpha 0.19 on average, and its
  square 0.037 to 0.040 over the 256 and 512 square maps.
- Some of Skate 3's skin diffuse maps (male legs, female arms) are DXT1, with no alpha, which reads
  as 1. Every texel took the whole highlight of Skate 2's `skin` material, which their materials
  name.
- Other Skate 3 skin maps are DXT5 with a mask of their own, such as female legs (0.026). Its cloth
  is DXT5 too (about 0.03).
- Port: a skin material whose diffuse has no alpha (DXT1, R5G6B5) takes 0.038 as its mask, the mean
  of Skate 2's (`skin_specular_mask`, `SKATE2_SKIN_SPECULAR_MASK`). Every other material keeps its
  own.
- Seen in Edit Skater: a male skater's legs under Skate 3's shorts lose the bright, wet highlight
  they had and take the sheen of Skate 2's skin, and a female skater's Skate 3 arms (DXT1, alpha
  1) under a Skate 3 tee lose the white highlights along the forearm and fingers.
  `SKATE2_DIAGNOSTIC_SKIN_MASK_OFF` gives every skin its diffuse alpha's mask again, for
  comparison.
- `SKATE2_DIAGNOSTIC_CAS_TEXTURES` logs each CAS material's maps with their format, size, mean alpha
  and mean alpha squared.

## Helper bones

- Skate 3's rig has a helper at each shoulder and hip joint: `LeftShoulderHlp`,
  `RightShoulderHlp`, `LeftUpLegHlp` and `RightUpLegHlp`.
  - Its character animators keep their indices beside the arm, shoulder, thigh and twist bones
    (`CASCharacterAnimator::Init` 0x82533ca8, `SkaterPresentation::InitAnimation` 0x82787ea8).
  - Skate 2's hierarchy has none of them.
- A model's bone the hierarchy lacks skins on the hips. Skate 3's tops and arms skin shoulder
  vertices to the shoulder helpers, and its bottoms and legs skin hip vertices to the hip ones.
  Those vertices stayed with the hips as the spine, arms or thighs moved.
- Port: each helper skins on its limb's bone (`LeftArm`, `RightArm`, `LeftUpLeg`, `RightUpLeg`)
  with that bone's own bind pose (`helper_bone_stand_in`). At the bind pose the vertices stand where
  they were.
  - `SKATE2_DIAGNOSTIC_S3_HELPER_PARENT` puts them on the joint's parent side instead (the shoulder,
    the hips), for comparison.
  - In Edit Skater's seated pose the two bindings look the same.

## Skate 2's head under Skate 3's tops

- On a female skater, Skate 3's Trefoil Tee and Customizable Tee showed skin through the top of
  the left shoulder, by the collar. It was the head's neck (`Rostral`, `female_thin_head`):
  - hidden (`SKATE2_DIAGNOSTIC_CAS_HIDE=Rostral`), the patch was gone;
  - hiding the arm, or Skate 2's arms, left it as it was;
  - the tee is one `cacstamp_cloth_ropa` mesh with no skin of its own.
- Ruled out as well:
  - the helper bindings: the patch was the same with each;
  - the cloth: `SKATE2_DIAGNOSTIC_S3_NO_CLOTH` skins Skate 3's clothing with no cloth, and the
    patch stayed;
  - the bind poses: of the tee's 36 bones, 9 have an inverse bind other than the shared one, in
    rotation alone and by rounding; skinning each with its own changed nothing.
- Skate 3 remade the heads and eyes, as it did the arms, and made its tops on them:

  | Part | Skate 2 model | Skate 3 model |
  | --- | --- | --- |
  | `female_thin_head` | `0xd44094cf18159ddd` | `0xf2fdb9aa8ec6271c` |
  | `head_mesh` (male) | `0xaea6737d633008ec` | `0x9461df2eb3d482dc` |
  | `eyes_female` | `0xaffc7fbae3ea33f2` | `0xdbe5ecdf58eb9a5a` |
  | `eyes` (male) | `0xfb1cc67a91f93031` | `0xb278a4bfb3d6c4bc` |
  | `shortsleeve_tshirt_female` (the Trefoil Tee's) | `0x8ffdd4a1c27adf10` | `0xc533893dc2e8db16` |

- Skate 3's head and eyes under the tee (tried, then taken out) close the collar, but they are
  another face: its own nose and mouth, without the lip colour and eyebrows Skate 2's face
  materials give, white about the eyes. Its mesh also carries more blend targets than Skate 2's
  21 zones. The port keeps the skater's own head and eyes.
  - `GetRostralID` (0x82ddca28) matches the same six attributes in both games: nothing ties the
    head to the top worn.
- Port (`crate::skate3::head_fit`, as each character scene loads): under a Skate 3 top (an
  `OuterTorso` or `InnerTorso` whose model names one of Skate 3's helper bones), each head
  vertex out of the top's surface (along its normal, inward, within 25 mm) or less than 4 mm
  under it moves in along its normal to 4 mm under the surface. Vertices at one place move
  together, along their normals' mean. Skin the top does not cover stays where it is. Each level
  of detail of the head fits under the tops' level of the same level, else their nearest.
  - Measured (bind pose): a female head's level 0 under the Trefoil Tee has 22 vertices out of it
    and 30 within the margin, the farthest moved 22.6 mm; its level 2, 1 and 12. A male head's
    has none out and 5 within the margin, moved 1.8 mm at most.
  - Seen in Edit Skater: the patch is gone under the Trefoil Tee and the Customizable Tee, with
    the face and neck as they were.
  - `SKATE2_DIAGNOSTIC_S3_HEAD_FIT=off` leaves the head as it is; any other value logs each
    level of detail fitted.
- On a male skater, the Trefoil Tee over Skate 2's head showed no such patch.
- `SKATE2_DIAGNOSTIC_CAS_HIDE=<component>[:<effect>],...` leaves those meshes undrawn, and lists
  every part's meshes with their effects.
