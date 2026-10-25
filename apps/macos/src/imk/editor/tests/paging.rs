//! 用真实候选列表翻页后数字选词，防止第二页仍误选第一页同序号。

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_predict::live::FocusedDraftModel;

use crate::imk::editor::keyboard::digit_index;

#[test]
fn digits_select_the_highlighted_page_in_a_real_candidate_list() {
    let dictionary = Dictionary::parse("你\tni\t20000\n呢\tni\t19000\n泥\tni\t18000\n拟\tni\t17000\n逆\tni\t16000\n妮\tni\t15000\n腻\tni\t14000\n倪\tni\t13000\n霓\tni\t12000\n匿\tni\t11000\n溺\tni\t10000\n昵\tni\t9000\n").unwrap();
    let mut engine = Engine::new(dictionary);
    for digit in [1, 2, 3] {
        let mut model = FocusedDraftModel::new();
        model.text(&mut engine, "ni", false, true);
        assert!(model.candidates().len() >= 12);
        model.move_highlight(9);
        assert_eq!(model.highlighted(), 9);
        let index = digit_index(model.highlighted(), digit, model.candidates().len()).unwrap();
        assert_eq!(index, 8 + digit);
        let expected = model.candidates()[index].text.clone();
        let first_page = model.candidates()[digit - 1].text.clone();
        assert_ne!(expected, first_page);
        assert!(model.choose(&mut engine, index));
        assert_eq!(model.committed(), expected);
        assert_eq!(model.original_output(), Some(expected));
        assert!(engine.composition().is_empty());
    }
}

#[test]
fn page_digits_reject_zero_ten_and_empty_slots() {
    assert_eq!(digit_index(9, 1, 10), Some(9));
    assert_eq!(digit_index(9, 2, 10), None);
    assert_eq!(digit_index(0, 0, 12), None);
    assert_eq!(digit_index(0, 10, 12), None);
    assert_eq!(digit_index(0, 1, 0), None);
}

#[test]
fn keyboard_digit_branch_consumes_empty_page_slot_without_selecting_first_page() {
    let dictionary = Dictionary::parse("你\tni\t20000\n呢\tni\t19000\n泥\tni\t18000\n拟\tni\t17000\n逆\tni\t16000\n妮\tni\t15000\n腻\tni\t14000\n倪\tni\t13000\n霓\tni\t12000\n匿\tni\t11000\n").unwrap();
    let mut engine = Engine::new(dictionary);
    let mut model = FocusedDraftModel::new();
    model.text(&mut engine, "ni", false, true);
    assert_eq!(model.candidates().len(), 10);
    model.move_highlight(9);
    let source = model.source();
    let revision = model.revision();
    assert!(crate::imk::editor::keyboard::handle_digit(
        &mut model,
        &mut engine,
        "2",
        true
    ));
    assert_eq!(model.committed(), "");
    assert_eq!(model.raw_keys(), "ni");
    assert_eq!(model.highlighted(), 9);
    assert_eq!(model.source(), source);
    assert_eq!(model.revision(), revision);
    let expected = model.candidates()[9].text.clone();
    assert!(crate::imk::editor::keyboard::handle_digit(
        &mut model,
        &mut engine,
        "1",
        true
    ));
    assert_eq!(model.committed(), expected);
}

#[test]
fn keyboard_digits_keep_core_expression_raw_and_unicode_input() {
    for prefix in ["v1+", "no-way", "u4e0"] {
        let mut engine = Engine::new(Dictionary::parse("").unwrap());
        engine.set_input(prefix);
        let mut model = FocusedDraftModel::new();
        model.refresh(&mut engine);
        assert!(!crate::imk::editor::keyboard::handle_digit(
            &mut model,
            &mut engine,
            "1",
            true
        ));
        model.text(&mut engine, "1", false, true);
        assert_eq!(model.raw_keys(), format!("{prefix}1"));
        assert_eq!(model.committed(), "");
    }
}
