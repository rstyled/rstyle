"""Calls that pass arguments by keyword."""


def padded(xs: list[int], ys: list[int]) -> int:
    """Scans ys for each of xs, then prints with a keyword."""
    total = 0
    for x in xs:
        for y in ys:
            total += x * y
    print(total, end="")
    return total


def ordered(xs: list[int]) -> list[int]:
    """Hands sorted a flag by keyword."""
    return sorted(xs, reverse=True)


def joined(names: list[str]) -> str:
    """Calls a method with a keyword."""
    return ", ".join(names).encode(errors="replace").decode()


def keyed(xs: list[int]) -> list[int]:
    """Hands sorted a lambda by keyword, which nothing lowers."""
    return sorted(xs, key=lambda x: -x)


def spread(xs: list[int], options: dict[str, bool]) -> list[int]:
    """Unpacks a mapping into keywords, which stays unmodeled."""
    return sorted(xs, **options)
