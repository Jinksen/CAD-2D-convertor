from cad2maxwell_backend.formatting import format_number, format_quantity


def test_quantities_use_a_no_break_space_before_the_unit() -> None:
    assert format_quantity(5, "mm²") == "5 mm²"
    assert format_quantity(12.34567, "mm") == "12.35 mm"


def test_small_and_large_values_use_powers_of_ten() -> None:
    assert format_number(1e-4) == "1 × 10⁻⁴"
    assert format_number(2.5e-7) == "2.5 × 10⁻⁷"
    assert format_number(1234567) == "1.235 × 10⁶"
    assert format_number(0.001) == "0.001"


def test_rounding_into_the_next_decade_and_signs() -> None:
    assert format_number(9.99996e-5) == "1 × 10⁻⁴"
    assert format_number(-0.5) == "−0.5"
    assert format_number(0) == "0"
