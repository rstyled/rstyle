"""Loops over dicts and sets that nothing in the function can change, and over ones it does."""


def pairs(counts: dict[str, int]) -> int:
    """Scans a dict's items."""
    total = 0
    for key, value in counts.items():
        total += value
    return total


def values(counts: dict[str, int]) -> int:
    """Scans a dict's values."""
    total = 0
    for value in counts.values():
        total += value
    return total


def members(seen: set[int]) -> int:
    """Scans a set."""
    total = 0
    for item in seen:
        total += item
    return total


def grown(counts: dict[str, int]) -> int:
    """Adds an entry before scanning, so the scan sees more than the dict held where the function started."""
    counts.update({"extra": 0})
    total = 0
    for value in counts.values():
        total += value
    return total
