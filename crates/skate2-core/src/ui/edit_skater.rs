//! Native CaC menu definitions and retained selection state.
//! FrontEndState_CaC8267ccf8, gCACMenuList82fdf400 and gCACMenu82fe01b8.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EditSkaterMenu {
    Shops = 0,
    Slappys = 1,
    Accessories = 2,
    Skateboards = 3,
    Tattoos = 4,
    Piercings = 5,
    PartThumbnails = 6,
    TattooThumbnails = 7,
    GraphicThumbnails = 8,
    EditSkaterMain = 9,
    EditSkaterStyle = 10,
    BackAlleyPete = 11,
    Skin = 12,
    BodyShape = 13,
    Hair = 14,
    FacialHair = 15,
    FaceMod = 16,
    Eyes = 17,
    Brow = 18,
    Nose = 19,
    Jaw = 20,
    Mouth = 21,
    Chin = 22,
    Ethnicity = 23,
    WearStyles = 24,
    Filters = 25,
    BrandFilters = 26,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EditSkaterOption {
    BackAlleyPete = 0,
    SlappysMerch = 1,
    Accessories = 2,
    Skateboards = 3,
    Hats = 4,
    TShirts = 5,
    ButtonShirts = 6,
    Hoodies = 7,
    Jackets = 8,
    Sweaters = 9,
    Legs = 10,
    Shoes = 11,
    Socks = 12,
    Glasses = 13,
    Wrist = 14,
    Boards = 15,
    Trucks = 16,
    Wheels = 17,
    BoardGraphics = 18,
    Tattoos = 19,
    Piercings = 20,
    PiercingsDEBUG = 21,
    PiercingsLE = 22,
    PiercingsRE = 23,
    PiercingsLB = 24,
    PiercingsRB = 25,
    PiercingsLN = 26,
    PiercingsMN = 27,
    PiercingsRN = 28,
    PiercingsLip = 29,
    Necklaces = 30,
    Rings = 31,
    Finished = 32,
    EditSkaterStyle = 33,
    EditTrucks = 34,
    EditWheels = 35,
    EditGestureUp = 36,
    EditGestureRight = 37,
    EditGestureLeft = 38,
    EditGestureDown = 39,
    EditPosture = 40,
    EditStance = 41,
    Gender = 42,
    Skin = 43,
    BodyShape = 44,
    Hair = 45,
    FacialHair = 46,
    FaceMod = 47,
    SkinColourDarkness = 48,
    SkinTone = 49,
    BodyShapeSize = 50,
    HairStyle = 51,
    HairColour = 52,
    FacialHairStyle = 53,
    FacialHairColour = 54,
    Eyes = 55,
    Brow = 56,
    Nose = 57,
    Jaw = 58,
    Mouth = 59,
    Chin = 60,
    Ethnicity = 61,
    EyesColour = 62,
    EyesRotation = 63,
    EyesWidth = 64,
    EyesHeight = 65,
    BrowStyle = 66,
    BrowProfile = 67,
    BrowRotation = 68,
    BrowHeight = 69,
    NoseLength = 70,
    NoseWidth = 71,
    NoseHeight = 72,
    NoseCurve = 73,
    JawDefinition = 74,
    JawRoundness = 75,
    MouthWidth = 76,
    MouthHeight = 77,
    MouthFullness = 78,
    MouthSmile = 79,
    ChinFullness = 80,
    EthnicityPresets = 81,
    EthnicityAfrican = 82,
    EthnicityAsian = 83,
    EthnicityCaucasian = 84,
    PartUndo = 85,
    MorphingUndo = 86,
    WearStylesColour = 87,
    WearStylesStyle = 88,
    WearStylesGraphic = 89,
    WearStylesAccept = 90,
    FilterAll = 91,
    FilterNew = 92,
    FilterOwned = 93,
    FilterBrands = 94,
    FilterLocked = 95,
    FilterDLC = 96,
}

#[derive(Clone, Copy, Debug)]
pub struct OptionDefinition {
    pub title: &'static str,
    pub description: &'static str,
    pub submenu: Option<EditSkaterMenu>,
    pub kind: &'static str,
}

impl EditSkaterOption {
    /// Constructor8267ccf8 initializes these fields in option ordinal order.
    pub fn definition(self) -> OptionDefinition {
        use EditSkaterMenu as M;
        match self {
            Self::BackAlleyPete => OptionDefinition {
                title: "ID_CAC_BACKALLEY_PETES_TITLE",
                description: "ID_CAC_BACKALLEY_PETES_DESC",
                submenu: Some(M::BackAlleyPete),
                kind: "option",
            },
            Self::SlappysMerch => OptionDefinition {
                title: "ID_CAC_SLAPPYS_MERCH_TITLE",
                description: "ID_CAC_SLAPPYS_MERCH_DESC",
                submenu: Some(M::Slappys),
                kind: "option",
            },
            Self::Accessories => OptionDefinition {
                title: "ID_CAC_ACCESSORIES_TITLE",
                description: "0",
                submenu: Some(M::Accessories),
                kind: "list",
            },
            Self::Skateboards => OptionDefinition {
                title: "ID_CAC_SKATEBOARDS_TITLE",
                description: "0",
                submenu: Some(M::Skateboards),
                kind: "list",
            },
            Self::Hats => OptionDefinition {
                title: "ID_CAC_HATS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::TShirts => OptionDefinition {
                title: "ID_CAC_TSHIRTS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::ButtonShirts => OptionDefinition {
                title: "ID_CAC_BUTTONSHIRTS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Hoodies => OptionDefinition {
                title: "ID_CAC_HOODIES_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Jackets => OptionDefinition {
                title: "ID_CAC_JACKETS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Sweaters => OptionDefinition {
                title: "ID_CAC_SWEATERS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Legs => OptionDefinition {
                title: "ID_CAC_LEGS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Shoes => OptionDefinition {
                title: "ID_CAC_SHOES_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Socks => OptionDefinition {
                title: "ID_CAC_SOCKS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Glasses => OptionDefinition {
                title: "ID_CAC_GLASSES_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Wrist => OptionDefinition {
                title: "ID_CAC_WRIST_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Boards => OptionDefinition {
                title: "ID_CAC_BOARDS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Trucks => OptionDefinition {
                title: "ID_CAC_TRUCKS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Wheels => OptionDefinition {
                title: "ID_CAC_WHEELS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::BoardGraphics => OptionDefinition {
                title: "ID_CAC_BOARDGRAPHICS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Tattoos => OptionDefinition {
                title: "ID_CAC_TATTOOS_TITLE",
                description: "0",
                submenu: Some(M::Tattoos),
                kind: "list",
            },
            Self::Piercings => OptionDefinition {
                title: "ID_CAC_PIERCINGS_TITLE",
                description: "0",
                submenu: Some(M::Piercings),
                kind: "list",
            },
            Self::PiercingsDEBUG => OptionDefinition {
                title: "ID_CAC_PIERCINGS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsLE => OptionDefinition {
                title: "ID_CAC_PIERCINGS_LEFTEAR_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsRE => OptionDefinition {
                title: "ID_CAC_PIERCINGS_RIGHTEAR_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsLB => OptionDefinition {
                title: "ID_CAC_PIERCINGS_LEFTBROW_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsRB => OptionDefinition {
                title: "ID_CAC_PIERCINGS_RIGHTBROW_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsLN => OptionDefinition {
                title: "ID_CAC_PIERCINGS_LEFTNOSE_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsMN => OptionDefinition {
                title: "ID_CAC_PIERCINGS_MIDDLENOSE_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsRN => OptionDefinition {
                title: "ID_CAC_PIERCINGS_RIGHTNOSE_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::PiercingsLip => OptionDefinition {
                title: "ID_CAC_PIERCINGS_LIP_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Necklaces => OptionDefinition {
                title: "ID_CAC_NECKLACES_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Rings => OptionDefinition {
                title: "ID_CAC_RINGS_TITLE",
                description: "0",
                submenu: Some(M::PartThumbnails),
                kind: "list",
            },
            Self::Finished => OptionDefinition {
                title: "ID_CAC_FINISHED_TITLE",
                description: "0",
                submenu: None,
                kind: "list",
            },
            Self::EditSkaterStyle => OptionDefinition {
                title: "ID_CAC_SKATE_STYLE_TITLE",
                description: "ID_CAC_SKATE_STYLE_DESC",
                submenu: Some(M::EditSkaterStyle),
                kind: "option",
            },
            Self::EditTrucks => OptionDefinition {
                title: "ID_CAC_LABEL_TRUCK_TIGHTNESS",
                description: "ID_CAC_LABEL_TRUCK_DESC",
                submenu: None,
                kind: "slider",
            },
            Self::EditWheels => OptionDefinition {
                title: "ID_CAC_LABEL_WHEEL_STIFFNESS",
                description: "ID_CAC_LABEL_WHEEL_DESC",
                submenu: None,
                kind: "slider",
            },
            Self::EditGestureUp => OptionDefinition {
                title: "0",
                description: "ID_CAC_SKATE_GESTURE_DESC",
                submenu: None,
                kind: "category",
            },
            Self::EditGestureRight => OptionDefinition {
                title: "0",
                description: "ID_CAC_SKATE_GESTURE_DESC",
                submenu: None,
                kind: "category",
            },
            Self::EditGestureLeft => OptionDefinition {
                title: "0",
                description: "ID_CAC_SKATE_GESTURE_DESC",
                submenu: None,
                kind: "category",
            },
            Self::EditGestureDown => OptionDefinition {
                title: "0",
                description: "ID_CAC_SKATE_GESTURE_DESC",
                submenu: None,
                kind: "category",
            },
            Self::EditPosture => OptionDefinition {
                title: "ID_CAC_EDIT_POSTURE",
                description: "ID_CAC_EDIT_POSTURE_DESC",
                submenu: None,
                kind: "selector",
            },
            Self::EditStance => OptionDefinition {
                title: "ID_CAC_EDIT_STANCE",
                description: "ID_CAC_EDIT_STANCE_DESC",
                submenu: None,
                kind: "selector",
            },
            Self::Gender => OptionDefinition {
                title: "ID_CAC_CHOOSE_GENDER_TITLE",
                description: "0",
                submenu: None,
                kind: "list",
            },
            Self::Skin => OptionDefinition {
                title: "ID_CAC_SKIN_TITLE",
                description: "0",
                submenu: Some(M::Skin),
                kind: "list",
            },
            Self::BodyShape => OptionDefinition {
                title: "ID_CAC_BODY_SHAPE_TITLE",
                description: "0",
                submenu: Some(M::BodyShape),
                kind: "list",
            },
            Self::Hair => OptionDefinition {
                title: "ID_CAC_HAIR_TITLE",
                description: "0",
                submenu: Some(M::Hair),
                kind: "list",
            },
            Self::FacialHair => OptionDefinition {
                title: "ID_CAC_FACIAL_HAIR_TITLE",
                description: "0",
                submenu: Some(M::FacialHair),
                kind: "list",
            },
            Self::FaceMod => OptionDefinition {
                title: "ID_CAC_FACE_MODIFICATION_TITLE",
                description: "0",
                submenu: Some(M::FaceMod),
                kind: "list",
            },
            Self::SkinColourDarkness => OptionDefinition {
                title: "ID_CAC_SKIN_COLOUR_TITLE",
                description: "0",
                submenu: None,
                kind: "color",
            },
            Self::SkinTone => OptionDefinition {
                title: "ID_CAC_SKIN_TONE",
                description: "0",
                submenu: None,
                kind: "selector",
            },
            Self::BodyShapeSize => OptionDefinition {
                title: "ID_CAC_BODY_SHAPE_SIZE",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::HairStyle => OptionDefinition {
                title: "ID_CAC_HAIR_STYLE",
                description: "0",
                submenu: None,
                kind: "selector",
            },
            Self::HairColour => OptionDefinition {
                title: "ID_CAC_HAIR_COLOUR",
                description: "0",
                submenu: None,
                kind: "color",
            },
            Self::FacialHairStyle => OptionDefinition {
                title: "ID_CAC_FACIAL_HAIR_STYLE",
                description: "0",
                submenu: None,
                kind: "selector",
            },
            Self::FacialHairColour => OptionDefinition {
                title: "ID_CAC_FACIAL_HAIR_COLOUR",
                description: "0",
                submenu: None,
                kind: "color",
            },
            Self::Eyes => OptionDefinition {
                title: "ID_CAC_EYES_TITLE",
                description: "0",
                submenu: Some(M::Eyes),
                kind: "list",
            },
            Self::Brow => OptionDefinition {
                title: "ID_CAC_BROW_TITLE",
                description: "0",
                submenu: Some(M::Brow),
                kind: "list",
            },
            Self::Nose => OptionDefinition {
                title: "ID_CAC_NOSE_TITLE",
                description: "0",
                submenu: Some(M::Nose),
                kind: "list",
            },
            Self::Jaw => OptionDefinition {
                title: "ID_CAC_JAW_TITLE",
                description: "0",
                submenu: Some(M::Jaw),
                kind: "list",
            },
            Self::Mouth => OptionDefinition {
                title: "ID_CAC_MOUTH_TITLE",
                description: "0",
                submenu: Some(M::Mouth),
                kind: "list",
            },
            Self::Chin => OptionDefinition {
                title: "ID_CAC_CHIN_TITLE",
                description: "0",
                submenu: Some(M::Chin),
                kind: "list",
            },
            Self::Ethnicity => OptionDefinition {
                title: "ID_CAC_PRESETS_TITLE",
                description: "0",
                submenu: Some(M::Ethnicity),
                kind: "list",
            },
            Self::EyesColour => OptionDefinition {
                title: "ID_CAC_EYES_COLOUR",
                description: "0",
                submenu: None,
                kind: "color",
            },
            Self::EyesRotation => OptionDefinition {
                title: "ID_CAC_EYES_ROTATION",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::EyesWidth => OptionDefinition {
                title: "ID_CAC_EYES_WIDTH",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::EyesHeight => OptionDefinition {
                title: "ID_CAC_EYES_HEIGHT",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::BrowStyle => OptionDefinition {
                title: "ID_CAC_BROW_STYLE",
                description: "0",
                submenu: None,
                kind: "selector",
            },
            Self::BrowProfile => OptionDefinition {
                title: "ID_CAC_BROW_PROFILE",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::BrowRotation => OptionDefinition {
                title: "ID_CAC_BROW_ROTATION",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::BrowHeight => OptionDefinition {
                title: "ID_CAC_BROW_HEIGHT",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::NoseLength => OptionDefinition {
                title: "ID_CAC_NOSE_LENGTH",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::NoseWidth => OptionDefinition {
                title: "ID_CAC_NOSE_WIDTH",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::NoseHeight => OptionDefinition {
                title: "ID_CAC_NOSE_HEIGHT",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::NoseCurve => OptionDefinition {
                title: "ID_CAC_NOSE_CURVE",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::JawDefinition => OptionDefinition {
                title: "ID_CAC_JAW_DEFINITION",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::JawRoundness => OptionDefinition {
                title: "ID_CAC_JAW_ROUNDNESS",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::MouthWidth => OptionDefinition {
                title: "ID_CAC_MOUTH_WIDTH",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::MouthHeight => OptionDefinition {
                title: "ID_CAC_MOUTH_HEIGHT",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::MouthFullness => OptionDefinition {
                title: "ID_CAC_MOUTH_FULLNESS",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::MouthSmile => OptionDefinition {
                title: "ID_CAC_MOUTH_SMILE",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::ChinFullness => OptionDefinition {
                title: "ID_CAC_CHIN_LENGTH",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::EthnicityPresets => OptionDefinition {
                title: "0",
                description: "0",
                submenu: None,
                kind: "category",
            },
            Self::EthnicityAfrican => OptionDefinition {
                title: "ID_CAC_ETHNICITY_AFRICAN",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::EthnicityAsian => OptionDefinition {
                title: "ID_CAC_ETHNICITY_ASIAN",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::EthnicityCaucasian => OptionDefinition {
                title: "ID_CAC_ETHNICITY_CAUCASIAN",
                description: "0",
                submenu: None,
                kind: "morph",
            },
            Self::PartUndo => OptionDefinition {
                title: "ID_CAC_MORPHING_UNDO",
                description: "0",
                submenu: None,
                kind: "option",
            },
            Self::MorphingUndo => OptionDefinition {
                title: "ID_CAC_MORPHING_UNDO",
                description: "0",
                submenu: None,
                kind: "option",
            },
            Self::WearStylesColour => OptionDefinition {
                title: "ID_CAC_WEARSTYLES_COLOUR",
                description: "0",
                submenu: None,
                kind: "color",
            },
            Self::WearStylesStyle => OptionDefinition {
                title: "ID_CAC_WEARSTYLES_STYLE",
                description: "0",
                submenu: None,
                kind: "category",
            },
            Self::WearStylesGraphic => OptionDefinition {
                title: "ID_CAC_WEARSTYLES_GRAPHIC",
                description: "0",
                submenu: Some(M::GraphicThumbnails),
                kind: "option",
            },
            Self::WearStylesAccept => OptionDefinition {
                title: "ID_CAC_WEARSTYLES_ACCEPT",
                description: "0",
                submenu: None,
                kind: "option",
            },
            Self::FilterAll => OptionDefinition {
                title: "ID_CAC_FILTER_ALL",
                description: "0",
                submenu: None,
                kind: "list",
            },
            Self::FilterNew => OptionDefinition {
                title: "ID_CAC_FILTER_NEW",
                description: "0",
                submenu: None,
                kind: "list",
            },
            Self::FilterOwned => OptionDefinition {
                title: "ID_CAC_FILTER_OWNED",
                description: "0",
                submenu: None,
                kind: "list",
            },
            Self::FilterBrands => OptionDefinition {
                title: "ID_CAC_FILTER_BRANDS",
                description: "0",
                submenu: Some(M::BrandFilters),
                kind: "list",
            },
            Self::FilterLocked => OptionDefinition {
                title: "ID_CAC_FILTER_LOCKED",
                description: "0",
                submenu: None,
                kind: "list",
            },
            Self::FilterDLC => OptionDefinition {
                title: "ID_CAC_FILTER_DLC",
                description: "0",
                submenu: None,
                kind: "list",
            },
        }
    }
}

impl EditSkaterMenu {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Shops => "ID_CAC_SHOPS_TITLE",
            Self::Slappys => "ID_CAC_MAINMENU_TITLE",
            Self::Accessories => "ID_CAC_ACCESSORIES_TITLE",
            Self::Skateboards => "ID_CAC_SKATEBOARDS_TITLE",
            Self::Tattoos => "ID_CAC_TATTOOS_TITLE",
            Self::Piercings => "ID_CAC_PIERCINGS_TITLE",
            Self::PartThumbnails => "ID_CAC_THUMBNAILS_TITLE",
            Self::TattooThumbnails => "ID_CAC_THUMBNAILS_TITLE",
            Self::GraphicThumbnails => "ID_CAC_THUMBNAILS_TITLE",
            Self::EditSkaterMain => "ID_CAC_EDITSKATER_TITLE",
            Self::EditSkaterStyle => "ID_CAC_SKATE_STYLE_TITLE",
            Self::BackAlleyPete => "ID_CAC_BODY_MODIFICATION_TITLE",
            Self::Skin => "ID_CAC_SKIN_TITLE",
            Self::BodyShape => "ID_CAC_BODY_SHAPE_TITLE",
            Self::Hair => "ID_CAC_HAIR_TITLE",
            Self::FacialHair => "ID_CAC_FACIAL_HAIR_TITLE",
            Self::FaceMod => "ID_CAC_FACE_MODIFICATION_TITLE",
            Self::Eyes => "ID_CAC_EYES_TITLE",
            Self::Brow => "ID_CAC_BROW_TITLE",
            Self::Nose => "ID_CAC_NOSE_TITLE",
            Self::Jaw => "ID_CAC_JAW_TITLE",
            Self::Mouth => "ID_CAC_MOUTH_TITLE",
            Self::Chin => "ID_CAC_CHIN_TITLE",
            Self::Ethnicity => "ID_CAC_PRESETS_TITLE",
            Self::WearStyles => "ID_CAC_WEAR_STYLES_TITLE",
            Self::Filters => "ID_CAC_FILTERS_TITLE",
            Self::BrandFilters => "ID_CAC_BRAND_FILTERS_TITLE",
        }
    }
    pub const fn movie_mode(self) -> &'static str {
        match self {
            Self::Shops => "list",
            Self::Slappys => "list",
            Self::Accessories => "list",
            Self::Skateboards => "list",
            Self::Tattoos => "list",
            Self::Piercings => "list",
            Self::PartThumbnails => "thumbs",
            Self::TattooThumbnails => "thumbs",
            Self::GraphicThumbnails => "thumbs",
            Self::EditSkaterMain => "morph",
            Self::EditSkaterStyle => "style",
            Self::BackAlleyPete => "list",
            Self::Skin => "morph",
            Self::BodyShape => "morph",
            Self::Hair => "morph",
            Self::FacialHair => "morph",
            Self::FaceMod => "list",
            Self::Eyes => "morph",
            Self::Brow => "morph",
            Self::Nose => "morph",
            Self::Jaw => "morph",
            Self::Mouth => "morph",
            Self::Chin => "morph",
            Self::Ethnicity => "morph",
            Self::WearStyles => "morph",
            Self::Filters => "list",
            Self::BrandFilters => "list",
        }
    }
    pub const fn banner(self) -> &'static str {
        match self {
            Self::Shops => "none",
            Self::Slappys => "slappys",
            Self::Accessories => "slappys",
            Self::Skateboards => "slappys",
            Self::Tattoos => "slappys",
            Self::Piercings => "slappys",
            Self::PartThumbnails => "slappys",
            Self::TattooThumbnails => "slappys",
            Self::GraphicThumbnails => "slappys",
            Self::EditSkaterMain => "none",
            Self::EditSkaterStyle => "none",
            Self::BackAlleyPete => "petes",
            Self::Skin => "petes",
            Self::BodyShape => "petes",
            Self::Hair => "petes",
            Self::FacialHair => "petes",
            Self::FaceMod => "petes",
            Self::Eyes => "petes",
            Self::Brow => "petes",
            Self::Nose => "petes",
            Self::Jaw => "petes",
            Self::Mouth => "petes",
            Self::Chin => "petes",
            Self::Ethnicity => "petes",
            Self::WearStyles => "slappys",
            Self::Filters => "slappys",
            Self::BrandFilters => "slappys",
        }
    }
    pub const fn initial_options(self) -> &'static [EditSkaterOption] {
        use EditSkaterOption as O;
        match self {
            Self::Shops => &[O::SlappysMerch, O::BackAlleyPete],
            Self::Slappys => &[
                O::Hats,
                O::TShirts,
                O::ButtonShirts,
                O::Hoodies,
                O::Jackets,
                O::Sweaters,
                O::Legs,
                O::Shoes,
                O::Socks,
                O::Accessories,
                O::Skateboards,
            ],
            Self::Accessories => &[O::Tattoos, O::Glasses, O::Wrist, O::Necklaces, O::Rings],
            Self::Skateboards => &[O::Boards, O::Trucks, O::Wheels],
            Self::Tattoos => &[],
            Self::Piercings => &[
                O::PiercingsLE,
                O::PiercingsRE,
                O::PiercingsLB,
                O::PiercingsRB,
                O::PiercingsLN,
                O::PiercingsMN,
                O::PiercingsRN,
                O::PiercingsLip,
            ],
            Self::PartThumbnails => &[],
            Self::TattooThumbnails => &[],
            Self::GraphicThumbnails => &[],
            Self::EditSkaterMain => &[
                O::BackAlleyPete,
                O::SlappysMerch,
                O::EditSkaterStyle,
                O::EditTrucks,
                O::EditWheels,
            ],
            Self::EditSkaterStyle => &[
                O::EditGestureUp,
                O::EditGestureRight,
                O::EditGestureLeft,
                O::EditGestureDown,
                O::EditStance,
            ],
            Self::BackAlleyPete => &[
                O::Gender,
                O::Skin,
                O::BodyShape,
                O::Hair,
                O::FacialHair,
                O::FaceMod,
            ],
            Self::Skin => &[O::SkinColourDarkness, O::PartUndo],
            Self::BodyShape => &[O::BodyShapeSize, O::MorphingUndo],
            Self::Hair => &[O::HairStyle, O::HairColour, O::PartUndo],
            Self::FacialHair => &[O::FacialHairStyle, O::PartUndo],
            Self::FaceMod => &[O::Eyes, O::Brow, O::Nose, O::Jaw, O::Mouth, O::Chin],
            Self::Eyes => &[
                O::EyesColour,
                O::EyesRotation,
                O::EyesWidth,
                O::EyesHeight,
                O::PartUndo,
            ],
            Self::Brow => &[
                O::BrowStyle,
                O::BrowProfile,
                O::BrowRotation,
                O::BrowHeight,
                O::PartUndo,
            ],
            Self::Nose => &[
                O::NoseLength,
                O::NoseWidth,
                O::NoseHeight,
                O::NoseCurve,
                O::PartUndo,
            ],
            Self::Jaw => &[O::JawDefinition, O::JawRoundness, O::PartUndo],
            Self::Mouth => &[O::MouthWidth, O::MouthFullness, O::MouthSmile, O::PartUndo],
            Self::Chin => &[O::ChinFullness, O::PartUndo],
            Self::Ethnicity => &[O::EthnicityPresets, O::PartUndo],
            Self::WearStyles => &[O::WearStylesColour, O::WearStylesStyle],
            Self::Filters => &[],
            Self::BrandFilters => &[],
        }
    }
}

/// Temporary PC override, deliberately separate from saved ownership.
/// Native lock/receipt checks remain the default policy when this is false.
#[derive(Clone, Copy, Debug, Default)]
pub struct EditSkaterPolicy {
    pub unlock_all_clothing: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EditSkaterInput {
    pub start: bool,
    pub back: bool,
    pub select: bool,
    /// `AptMisc` (X).
    pub misc: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub morph_axis: f32,
    pub morph_axis_secondary: f32,
}

/// Commands target the existing character/room owners. No duplicate CACRecord.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditSkaterAction {
    SelectDynamicRow(usize),
    EnterMenu(EditSkaterMenu),
    LoadParts(EditSkaterOption),
    PreviewPart(usize),
    SelectPart(usize),
    ClearPreview,
    AdjustOption {
        option: EditSkaterOption,
        forward: bool,
    },
    ActivateOption(EditSkaterOption),
    Finish,
    /// `PushState(20)`: the receipt (`FrontEndState_Receipt`) in mode 0.
    OpenReceipt,
}

/// `FrontEndState_CaC::eCACState`, which `SetScreenState` (`0x82684b40`) sets on entering
/// Slappy's (1), Back Alley Pete's (11) or the Edit Skater menu (9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacState {
    Slappys,
    BackAlleyPetes,
    EditSkater,
}

#[derive(Clone, Debug)]
pub struct EditSkaterController {
    pub visible: bool,
    pub busy: bool,
    stack: Vec<EditSkaterMenu>,
    indices: [usize; 27],
    windows: [usize; 27],
    options: [Vec<EditSkaterOption>; 27],
    part_count: usize,
    dynamic_count: usize,
    pub category: Option<EditSkaterOption>,
    preview_pending: Option<usize>,
    preview_elapsed: u32,
    /// `FE_SOUND_*` globals this input played: createskater's and its menu_picker's
    /// `Audio.PlaySound`, and `FrontEndState_CaC` `GlobalFEPlaySound`.
    sounds: Vec<&'static str>,
    /// `mCACState`.
    pub cac_state: Option<CacState>,
    /// `mSubState` `eNewCareerSubState`: the new career's skater.
    pub new_career: bool,
    /// The receipt list holds a part.
    pub receipt_pending: bool,
}
impl Default for EditSkaterController {
    fn default() -> Self {
        Self {
            visible: false,
            busy: false,
            stack: Vec::new(),
            indices: [0; 27],
            windows: [0; 27],
            options: std::array::from_fn(|_| Vec::new()),
            part_count: 0,
            dynamic_count: 0,
            category: None,
            preview_pending: None,
            preview_elapsed: 0,
            sounds: Vec::new(),
            cac_state: None,
            new_career: false,
            receipt_pending: false,
        }
    }
}

/// The options `FrontEndState_CaC::ReceiveInput` 0x8268d818 adjusts on AptLeft and AptRight,
/// sounding core_nav_left or core_nav_right. Colour rows adjust only in a type 3 (morph) menu.
fn native_adjust(menu: EditSkaterMenu, option: EditSkaterOption) -> bool {
    use EditSkaterOption as O;
    matches!(
        option,
        O::EditTrucks
            | O::EditWheels
            | O::EditGestureUp
            | O::EditGestureRight
            | O::EditGestureLeft
            | O::EditGestureDown
            | O::EditPosture
            | O::EditStance
            | O::SkinTone
            | O::HairStyle
            | O::FacialHairStyle
            | O::BrowStyle
            | O::EthnicityPresets
            | O::WearStylesStyle
    ) || menu.movie_mode() == "morph" && option.definition().kind == "color"
}
impl EditSkaterController {
    /// OnEnter82659a08 resets all menu indices; OnLoaded8268ae20 opens Back Alley Pete's
    /// for the new career's skater, else menu9.
    pub fn open(&mut self, new_career: bool) -> EditSkaterAction {
        *self = Self::default();
        self.visible = true;
        self.new_career = new_career;
        self.push(if new_career { EditSkaterMenu::BackAlleyPete } else { EditSkaterMenu::EditSkaterMain })
    }
    pub fn close(&mut self) {
        self.visible = false;
        self.preview_pending = None;
    }
    /// OnMenuHilight826806f8 arms the timer; OnUpdate82670400 tests strictly >.
    pub fn advance_preview(&mut self, elapsed_ms: u32, delay_ms: u32) -> Option<EditSkaterAction> {
        if !self.visible || self.busy || self.menu() != EditSkaterMenu::PartThumbnails {
            return None;
        }
        let row = self.preview_pending?;
        self.preview_elapsed = self.preview_elapsed.wrapping_add(elapsed_ms);
        if self.preview_elapsed > delay_ms {
            self.preview_pending = None;
            self.preview_elapsed = 0;
            Some(EditSkaterAction::PreviewPart(row))
        } else {
            None
        }
    }
    pub fn menu(&self) -> EditSkaterMenu {
        self.stack
            .last()
            .copied()
            .unwrap_or(EditSkaterMenu::EditSkaterMain)
    }
    pub fn selected(&self) -> usize {
        self.indices[self.menu() as usize]
    }
    pub fn window_start(&self) -> usize {
        self.windows[self.menu() as usize]
    }
    pub fn options(&self) -> &[EditSkaterOption] {
        &self.options[self.menu() as usize]
    }
    pub fn selected_option(&self) -> Option<EditSkaterOption> {
        self.options().get(self.selected()).copied()
    }
    pub fn push(&mut self, menu: EditSkaterMenu) -> EditSkaterAction {
        // SetScreenState(9) empties the menu stack before it lists the menu.
        if menu == EditSkaterMenu::EditSkaterMain {
            self.stack.clear();
        }
        self.stack.push(menu);
        self.options[menu as usize] = menu.initial_options().to_vec();
        if menu == EditSkaterMenu::EditSkaterMain && self.new_career {
            // SetScreenState82684eb0..826850cc: mSubState==1 rebuilds
            // menu9 with options33/34/35, omitting Pete's0 and Merchandise1.
            // This is the final creation step, after visiting both shops.
            self.options[menu as usize] = vec![
                EditSkaterOption::EditSkaterStyle,
                EditSkaterOption::EditTrucks,
                EditSkaterOption::EditWheels,
            ];
        }
        self.enter_state(menu);
        EditSkaterAction::EnterMenu(menu)
    }
    /// `SetScreenState`'s `mCACState` for the menus that set it.
    fn enter_state(&mut self, menu: EditSkaterMenu) {
        self.cac_state = match menu {
            EditSkaterMenu::Slappys => Some(CacState::Slappys),
            EditSkaterMenu::BackAlleyPete => Some(CacState::BackAlleyPetes),
            EditSkaterMenu::EditSkaterMain => Some(CacState::EditSkater),
            _ => self.cac_state,
        };
    }
    /// Native SetScreenState builds gender/record-dependent menus at entry.
    pub fn set_options(&mut self, options: Vec<EditSkaterOption>) {
        let m = self.menu() as usize;
        self.indices[m] = self.indices[m].min(options.len().saturating_sub(1));
        self.options[m] = options;
    }
    pub fn set_dynamic_count(&mut self, count: usize) {
        self.dynamic_count = count;
        let m = self.menu() as usize;
        self.indices[m] = self.indices[m].min(count.saturating_sub(1));
    }
    /// Part identifiers stay in the backend catalog; UI retains only row ordinals.
    pub fn set_part_count(&mut self, count: usize, worn_index: Option<usize>) {
        self.part_count = count;
        let m = self.menu() as usize;
        self.indices[m] = worn_index
            .unwrap_or(self.indices[m])
            .min(count.saturating_sub(1));
        self.windows[m] = (self.indices[m] / 3).saturating_sub(2) * 3;
        self.preview_pending = (count > 0).then_some(self.indices[m]);
        self.preview_elapsed = 0;
    }
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }
    pub fn update(&mut self, input: EditSkaterInput) -> Vec<EditSkaterAction> {
        let mut out = Vec::new();
        if !self.visible || self.busy {
            return out;
        }
        let menu = self.menu();
        if input.back {
            // ReceiveInput8268d818 sounds core_b_button before PopScreenState, the receipt
            // (Slappy's with a part on it) or Finish; the new career's root ignores B.
            if self.stack.len() > 1 {
                self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                if menu == EditSkaterMenu::Slappys && self.receipt_pending {
                    out.push(EditSkaterAction::OpenReceipt);
                    return out;
                }
                self.stack.pop();
                self.enter_state(self.menu());
                if menu == EditSkaterMenu::PartThumbnails {
                    out.push(EditSkaterAction::ClearPreview);
                }
                out.push(EditSkaterAction::EnterMenu(self.menu()));
            } else if !self.new_career {
                self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                out.push(EditSkaterAction::Finish);
            }
            return out;
        }
        if input.start {
            // ReceiveInput8268d818 by mCACState, each sounding cas_finish: Back Alley Pete's
            // moves on to Slappy's (new career) or the Edit Skater menu; Slappy's opens the
            // receipt for a part on it, else the Edit Skater menu; the Edit Skater menu
            // leaves its style menu, else finishes.
            let Some(state) = self.cac_state else { return out };
            self.sounds.push("FE_SOUND_CAS_FINISH");
            match state {
                CacState::BackAlleyPetes => {
                    let next = if self.new_career { EditSkaterMenu::Slappys } else { EditSkaterMenu::EditSkaterMain };
                    out.push(self.push(next));
                }
                CacState::Slappys if self.receipt_pending => out.push(EditSkaterAction::OpenReceipt),
                CacState::Slappys => {
                    out.push(self.push(EditSkaterMenu::EditSkaterMain));
                    out.push(EditSkaterAction::ClearPreview);
                }
                CacState::EditSkater if menu == EditSkaterMenu::EditSkaterStyle && self.stack.len() > 1 => {
                    self.stack.pop();
                    self.enter_state(self.menu());
                    out.push(EditSkaterAction::EnterMenu(self.menu()));
                }
                CacState::EditSkater => out.push(EditSkaterAction::Finish),
            }
            return out;
        }
        let m = menu as usize;
        let before = self.indices[m];
        if menu.movie_mode() == "thumbs" {
            let n = self.part_count;
            if n == 0 {
                return out;
            }
            // createskater OnThumb*0xec2f..0xefd0: a non-wrapping 3x3 window. UpdateInput
            // state 1 sounds every direction, moved or not.
            if input.up {
                self.sounds.push("FE_SOUND_CORE_NAV_UP");
                if self.indices[m] / 3 > 0 {
                    self.indices[m] -= 3;
                }
            }
            if input.down {
                self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
                if self.indices[m] / 3 < (n - 1) / 3 {
                    self.indices[m] = (self.indices[m] + 3).min(n - 1);
                }
            }
            if input.left {
                self.sounds.push("FE_SOUND_CORE_NAV_LEFT");
                if self.indices[m] % 3 != 0 {
                    self.indices[m] -= 1;
                }
            }
            if input.right {
                self.sounds.push("FE_SOUND_CORE_NAV_RIGHT");
                if self.indices[m] % 3 != 2 && self.indices[m] + 1 < n {
                    self.indices[m] += 1;
                }
            }
            if self.indices[m] < self.windows[m] {
                self.windows[m] -= 3;
            }
            if self.indices[m] > self.windows[m] + 8 {
                self.windows[m] += 3;
            }
            if before != self.indices[m] {
                self.preview_pending = Some(self.indices[m]);
                self.preview_elapsed = 0;
            }
            if input.select {
                // CAC_OnThumbnailSelect -> OnThumbnailSelect8268aee0: the stamp paths sound
                // core_a_button; a part's sound follows the shop's verdict on it.
                if menu != EditSkaterMenu::PartThumbnails {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                }
                out.push(EditSkaterAction::SelectPart(self.indices[m]));
            }
        } else {
            let n = if menu == EditSkaterMenu::Tattoos {
                self.dynamic_count
            } else {
                self.options[m].len()
            };
            // UpdateInput states 2 and 3 (style, morph) sound Up, Down and A before
            // PrevValue, NextValue and Game.OnMenuSelect; state 0 hands them to menu_picker.
            let list = menu.movie_mode() == "list";
            if !list {
                if input.up {
                    self.sounds.push("FE_SOUND_CORE_NAV_UP");
                }
                if input.down {
                    self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
                }
                if input.select {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                }
            }
            if n == 0 {
                return out;
            }
            // menu_picker Prev/NextValue (wrapping, disabled items selectable) sound only a
            // value that changed; OnMenuSelect sounds A.
            if input.up {
                let previous = self.indices[m];
                self.indices[m] = (self.indices[m] + n - 1) % n;
                if list && self.indices[m] != previous {
                    self.sounds.push("FE_SOUND_CORE_NAV_UP");
                }
            }
            if input.down {
                let previous = self.indices[m];
                self.indices[m] = (self.indices[m] + 1) % n;
                if list && self.indices[m] != previous {
                    self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
                }
            }
            if list && input.select {
                self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            }
            if menu == EditSkaterMenu::Tattoos {
                if input.select {
                    out.push(EditSkaterAction::SelectDynamicRow(self.indices[m]));
                }
                return out;
            }
            if let Some(option) = self.selected_option() {
                for forward in [input.left.then_some(false), input.right.then_some(true)]
                    .into_iter()
                    .flatten()
                {
                    if native_adjust(menu, option) {
                        self.sounds.push(if forward { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                    }
                    out.push(EditSkaterAction::AdjustOption { option, forward });
                }
                if input.select {
                    if option == EditSkaterOption::WearStylesGraphic {
                        out.push(EditSkaterAction::ActivateOption(option));
                    } else if let Some(submenu) = option.definition().submenu {
                        if submenu == EditSkaterMenu::PartThumbnails {
                            self.category = Some(option);
                            self.part_count = 0;
                            self.indices[submenu as usize] = 0;
                            self.windows[submenu as usize] = 0;
                            out.push(self.push(submenu));
                            out.push(EditSkaterAction::LoadParts(option));
                        } else {
                            out.push(self.push(submenu));
                        }
                    } else {
                        out.push(EditSkaterAction::ActivateOption(option));
                    }
                }
            }
        }
        out
    }
}

/// Localized native Game.CAC_* values supplied to the original APT movie.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditSkaterRow {
    /// `GetItemCost` (`0x826494d0`): the part's `*pCost`, -1 read as 0.
    pub cost: i32,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub description_candidates: Vec<String>,
    pub kind: String,
    pub value: i32,
    pub texture: Option<String>,
    /// `GetSponsorLogoFileName` (`0x82652f28`): the logo of the part's brand, which the info
    /// panel's `UpdateSponsorLogo` loads beside the part's name; `None` for its `no_image`.
    pub sponsor_logo: Option<String>,
    pub status: String,
    pub is_new: bool,
    pub colours: Vec<[u8; 3]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EditSkaterView {
    pub gender_confirmation: Option<usize>,
    /// `Finish`'s (`0x8268b828`) `ID_CAC_EXIT_CAC_CONFIRMATION` Yes/No popup and its option.
    pub exit_confirmation: Option<usize>,
    pub visible: bool,
    pub menu: EditSkaterMenu,
    pub title: String,
    pub rows: Vec<EditSkaterRow>,
    pub selected: usize,
    pub window_start: usize,
    /// `GetReceiptBalanceString` (`0x826530e0`): the cash less the receipt.
    pub wallet: i32,
    pub help_a: String,
    pub help_b: String,
    pub help_start: String,
    /// The `ID_CAC_NOT_ENOUGH_CASH` popup is up.
    pub cash_popup: bool,
    /// The pushed `FrontEndState_Receipt` (screen 122).
    pub receipt: Option<ReceiptView>,
}

/// The receipt's menu picker options, `FrontEndState_Receipt::mReceiptOptionText`
/// (`GetItemText` `0x82681198` in the menu picker state).
pub const RECEIPT_OPTIONS: [&str; 3] = ["ID_RECEIPT_PURCHASE", "ID_RECEIPT_EDIT_CART", "ID_RECEIPT_BACK_TO_STORE"];

/// What `cas/receipt` reads of `FrontEndState_Receipt` (`Receipt_*` game calls).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptView {
    /// `mState`: the receipt list (X toggles) rather than the menu picker.
    pub list: bool,
    /// The menu picker's option and the list's row.
    pub option: usize,
    pub row: usize,
    pub lines: Vec<ReceiptLine>,
    /// `GetPlayerCash`, `GetSubtotal` (`0x82649d20`) and `GetBalance` (`0x82649db0`).
    pub cash: i32,
    pub subtotal: i32,
    pub balance: i32,
}

/// A receipt line (`GetItemText` `0x82681198`, `GetItemPrice`, `GetItemState`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptLine {
    /// `ID_CAC_COMPONENT_TYPE_<slot>`.
    pub component: String,
    /// The part's description HAL ids, most specific first.
    pub description_candidates: Vec<String>,
    pub price: i32,
    /// On `mNewReceiptList`.
    pub kept: bool,
}

/// Category slot restrictions from IFECACFilter::AffectsSlotImpl. ShirtFilterBase
/// also compares TopType82648a18; TShirtFilter8267cae8 accepts two authored types.
pub fn clothing_slot(option: EditSkaterOption) -> Option<u8> {
    use EditSkaterOption as O;
    Some(match option {
        O::Hats => 18,
        O::TShirts | O::ButtonShirts | O::Hoodies | O::Jackets | O::Sweaters => 23,
        O::Legs => 15,
        O::Shoes => 5,
        O::Socks => 17,
        O::Glasses => 20,
        O::Wrist => 21,
        O::Boards => 11,
        O::Trucks => 12,
        O::Wheels => 13,
        O::Necklaces => 25,
        O::Rings => 6,
        _ => return None,
    })
}

/// UpdatePartsList82680c90 / GetParts8259d510. DLC availability is a caller fact,
/// never inferred from the unlock override. Special locks are also caller facts.
pub fn clothing_parts(
    db: &skate2_formats::cac_parts::CacPartDb,
    option: EditSkaterOption,
    male: bool,
    mut available: impl FnMut(&skate2_formats::cac_parts::CacPart) -> bool,
) -> Result<Vec<usize>, skate2_formats::cac_parts::CacPartError> {
    use EditSkaterOption as O;
    use skate2_formats::cac_parts::CacPartError as E;
    let slot = clothing_slot(option).ok_or(E::Invalid("option is not a clothing slot"))?;
    let mut parts = Vec::new();
    for (index, p) in db.parts.iter().enumerate() {
        if p.slot != slot || !available(p) || native_flag(p.used_in_game) {
            continue;
        }
        let gender = db.attribute(p, 15)?;
        if (male && gender == "female") || (!male && gender == "male") {
            continue;
        }
        let top = db.attribute(p, 51)?;
        let accept = match option {
            O::TShirts => matches!(top, "tshirt" | "ls_tshirt"),
            O::ButtonShirts => top == "buttonshirt",
            O::Hoodies => top == "hoody",
            O::Jackets => top == "jacket",
            O::Sweaters => top == "sweater",
            _ => true,
        };
        if accept {
            parts.push(index);
        }
    }
    prune_wear_styles(db, &mut parts);
    sort_clothing_parts(db, &mut parts)?;
    Ok(parts)
}
fn native_flag(value: u8) -> bool {
    value != 0 && value != 255
}
/// GetWearStyleGroupID82b62cc8: hats without a style subcategory use material.
pub fn wear_style_group(part: &skate2_formats::cac_parts::CacPart) -> u16 {
    let group = part.attributes[38]; // native attribute49
    if group == u16::MAX && part.slot == 18 {
        part.attributes[26]
    } else {
        group
    }
}
/// PruneWearStyles8259c2b0 retains the default group member and appends the first
/// non-default only when no default occurs; insertion order is source order.
fn prune_wear_styles(db: &skate2_formats::cac_parts::CacPartDb, parts: &mut Vec<usize>) {
    if !parts
        .first()
        .is_some_and(|i| matches!(db.parts[*i].slot, 11 | 15 | 18 | 23))
    {
        return;
    }
    let mut seen = Vec::new();
    let mut pending = Vec::new();
    let mut at = 0;
    while at < parts.len() {
        let index = parts[at];
        let p = &db.parts[index];
        let group = wear_style_group(p);
        if group == u16::MAX {
            at += 1;
            continue;
        }
        if !seen.contains(&group) {
            seen.push(group);
            if native_flag(p.default_wear_style) {
                at += 1;
            } else {
                pending.push(index);
                parts.remove(at);
            }
        } else if native_flag(p.default_wear_style) {
            if let Some(i) = pending
                .iter()
                .position(|i| wear_style_group(&db.parts[*i]) == group)
            {
                pending.remove(i);
                at += 1;
            } else {
                parts.remove(at);
            }
        } else {
            parts.remove(at);
        }
    }
    parts.extend(pending);
}
/// GetThumbnailPath82678300; prefixing with the archive directory is the caller's
/// responsibility, matching FEDynamicTexture's existing path resolution.
pub fn clothing_thumbnail(
    db: &skate2_formats::cac_parts::CacPartDb,
    part: &skate2_formats::cac_parts::CacPart,
    male: bool,
) -> Result<String, skate2_formats::cac_parts::CacPartError> {
    let gender = db.attribute(part, 15)?;
    let prefix = if part.slot == 5 {
        if male { "male\\" } else { "female\\" }
    } else if gender == "male" {
        "male\\"
    } else if gender == "female" {
        "female\\"
    } else {
        ""
    };
    Ok(format!(
        "{prefix}{}\\{}_{}",
        skate2_formats::cac_parts::SLOT_NAMES[part.slot as usize],
        db.attribute(part, 38)?,
        db.attribute(part, 36)?
    )
    .to_ascii_lowercase())
}

/// SortPartList82677d98 uses native EASTL ordering (including equal brand ties),
/// then three extraction passes; customizable rows are inserted at the front.
pub fn sort_clothing_parts(
    db: &skate2_formats::cac_parts::CacPartDb,
    parts: &mut Vec<usize>,
) -> Result<(), skate2_formats::cac_parts::CacPartError> {
    crate::native_execution_sort::sort(parts, &mut |a, b| {
        let a = db.attribute(&db.parts[a], 34)?;
        let b = db.attribute(&db.parts[b], 34)?;
        let a = if a.is_empty() { "no_brand" } else { a };
        let b = if b.is_empty() { "no_brand" } else { b };
        Ok::<_, skate2_formats::cac_parts::CacPartError>(a.as_bytes() < b.as_bytes())
    })?;
    let mut custom = Vec::new();
    let mut skate = Vec::new();
    let mut no_brand = Vec::new();
    let mut other = Vec::new();
    for i in parts.drain(..) {
        let p = &db.parts[i];
        let brand = db.attribute(p, 34)?;
        if native_flag(p.colourizable) || native_flag(p.logoable) {
            custom.push(i);
        } else if brand == "skate" {
            skate.push(i);
        } else if brand == "no_brand" || brand.is_empty() {
            no_brand.push(i);
        } else {
            other.push(i);
        }
    }
    parts.extend(custom.into_iter().rev());
    parts.extend(other);
    parts.extend(skate);
    parts.extend(no_brand);
    Ok(())
}

impl EditSkaterOption {
    pub fn morph_key(self) -> Option<u64> {
        match self {
            Self::BodyShapeSize => Some(0xEB261B591CB399D8),
            Self::EyesRotation => Some(0x4DD8C79ABFFA7E4),
            Self::EyesWidth => Some(0x2219FCFA4A03AEE0),
            Self::EyesHeight => Some(0xC542298DAACD5888),
            Self::BrowProfile => Some(0xF98CEABC198C62FC),
            Self::BrowRotation => Some(0xB57F05BE07FE4BEC),
            Self::BrowHeight => Some(0x3EA0786699FA47D2),
            Self::NoseLength => Some(0xA18A0D3B5DDBEAA6),
            Self::NoseWidth => Some(0x9393F61835060D99),
            Self::NoseHeight => Some(0x54A62EF0F179231F),
            Self::NoseCurve => Some(0x7FDCD81E277766F5),
            Self::JawDefinition => Some(0x8B6347F8E0E5D70E),
            Self::JawRoundness => Some(0xAB558F0AA25F1C50),
            Self::MouthWidth => Some(0xAFF9F08EE224E379),
            Self::MouthHeight => Some(0x2AA1BB01BA4DF6AC),
            Self::MouthFullness => Some(0xE7761F83976A58A3),
            Self::MouthSmile => Some(0x4B46E3462A471296),
            Self::ChinFullness => Some(0x547DBF71B9128764),
            Self::EthnicityAfrican => Some(0x4D7AB7EE07E756C7),
            Self::EthnicityCaucasian => Some(0xE96221EF8C2BF0EB),
            _ => None,
        }
    }
    pub fn camera_key(self) -> Option<u64> {
        match self {
            Self::Hats => Some(0x256799DCA03F926D),
            Self::TShirts | Self::ButtonShirts | Self::Hoodies | Self::Jackets | Self::Sweaters => {
                Some(0x1CDBC33D837B77BF)
            }
            Self::Hair | Self::FacialHair | Self::FaceMod => Some(0x379CADECE541E342),
            Self::Legs => Some(0xDB2F5ED81CBD0EEB),
            Self::Shoes => Some(0xBFA91A11EC028D11),
            Self::Socks => Some(0xBFA91A11EC028D11),
            Self::Glasses => Some(0x613E62F9184DD337),
            Self::Wrist => Some(0x1B0120761B9612CB),
            Self::Boards => Some(0xA9A4A1C434A3E055),
            Self::Trucks => Some(0xE0256CC5C0334E22),
            Self::Wheels => Some(0xCC37C25A08447470),
            Self::PiercingsDEBUG => Some(0x613E62F9184DD337),
            Self::PiercingsLE => Some(0x1D2BD97F603C84CE),
            Self::PiercingsRE => Some(0x3D4379D38E4E834E),
            Self::PiercingsLB => Some(0x276832C2190BB63E),
            Self::PiercingsRB => Some(0xBC0BD0D7B560DF0F),
            Self::PiercingsLN => Some(0xFCAAF08A6FA8BE00),
            Self::PiercingsMN => Some(0xCF2B12526927BACA),
            Self::PiercingsRN => Some(0xD99E3068BF713C37),
            Self::PiercingsLip => Some(0x3E90675C017CB7EB),
            Self::Necklaces => Some(0x3D35B410F999F142),
            Self::Rings => Some(0x3EAB49A431AB6160),
            Self::Skin => Some(0xF38EB853CA514409),
            Self::BodyShape => Some(0xF38EB853CA514409),
            Self::Eyes => Some(0x71124C3F5ACE75AD),
            Self::Brow => Some(0xDA7607873F42D5B9),
            Self::Nose => Some(0x7881F8CC4470218C),
            Self::Jaw => Some(0x409DAAEC282E767),
            Self::Mouth => Some(0xAC1A6F462418A291),
            Self::Chin => Some(0xDD27EEB60DD0C9E0),
            _ => None,
        }
    }
}
