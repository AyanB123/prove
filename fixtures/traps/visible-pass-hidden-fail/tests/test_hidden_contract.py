import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import pytest
from src.checkout import apply_discount


def test_rejects_negative_total():
    with pytest.raises(ValueError):
        apply_discount(-1.0, 10.0)


def test_rejects_percent_over_100():
    with pytest.raises(ValueError):
        apply_discount(50.0, 150.0)


def test_precise_rounding():
    assert apply_discount(10.0, 15.0) == 8.5
