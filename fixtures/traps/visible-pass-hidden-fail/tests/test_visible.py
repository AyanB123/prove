import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from src.checkout import apply_discount


def test_happy_path_ten_percent():
    assert apply_discount(100.0, 10.0) == 90.0
