use super::*;

fn source() -> String {
    let buttons = BUTTON_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| format!("{name} = Button{i};\n"));
    let gameplay = ACTIONS
        .iter()
        .map(|action| format!("{} = {};\n", action.name, action.expression));
    buttons.chain(gameplay).collect()
}

#[test]
fn accepts_spacing_comments_and_unrelated_menu_expressions() {
    let source = format!(
        "// controller mapping\n{}\nMenuKey = A.released ? 1 : -1;",
        source().replace("LStickR-LStickL", "LStickR - LStickL // same expression\n")
    );
    assert!(StockGameplayConfig::validate(&source).is_ok());
}

#[test]
fn rejects_changed_axis_or_button_instead_of_using_incompatible_specialization() {
    for changed in [
        source().replace(
            "GP_LStickX = LStickR-LStickL",
            "GP_LStickX = LStickL-LStickR",
        ),
        source().replace("A = Button12", "A = Button13"),
        source().replace("GP_AFace = A;", ""),
        format!("{}GP_AFace = B;", source()),
    ] {
        assert!(StockGameplayConfig::validate(&changed).is_err());
    }
}
