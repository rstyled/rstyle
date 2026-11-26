"""Calls that bind their arguments other than where they're written: a class's, a class method's, a
static method's and a method's read through a class object. Each sink is reached by one call only."""

import subprocess


class OpensPath:
    def __init__(self, path: str, mode: str) -> None:
        self.handle = open(path)


class OpensData:
    def __init__(self, name: str, path: str) -> None:
        self.handle = open(path)


class OpensByKeyword:
    def __init__(self, name: str, path: str) -> None:
        self.handle = open(path)


class OpensNamed:
    def __init__(self, name: str, path: str) -> None:
        self.handle = open(path)


class Tools:
    @staticmethod
    def run(command: str) -> None:
        subprocess.run(command, shell=True)

    @staticmethod
    def pair(label: str, command: str) -> None:
        subprocess.run(command, shell=True)

    @classmethod
    def make(cls, label: str, command: str) -> "Tools":
        subprocess.run(command, shell=True)
        return cls()

    @classmethod
    def labeled(cls, label: str, command: str) -> "Tools":
        subprocess.run(command, shell=True)
        return cls()

    @classmethod
    def built(cls, label: str, command: str) -> "Tools":
        subprocess.run(command, shell=True)
        return cls()

    def again(self, command: str) -> None:
        subprocess.run(command, shell=True)

    def unbound(self) -> None:
        type(self).again(self, input())


def first_opened() -> None:
    OpensPath("a.txt", input())


def second_opened() -> None:
    OpensData("a.txt", input())


def second_by_keyword() -> None:
    OpensByKeyword(path=input(), name="a")


def first_by_keyword() -> None:
    OpensNamed(name=input(), path="a.txt")


def static_through_instance(tools: Tools) -> None:
    tools.run(input())


def static_label(tools: Tools) -> None:
    tools.pair(input(), "ls")


def class_method() -> None:
    Tools.make("label", input())


def class_method_label() -> None:
    Tools.labeled(input(), "ls")


def class_method_through_instance(tools: Tools) -> None:
    tools.built("label", input())


def popen_shell() -> None:
    subprocess.Popen(input(), shell=True)


def popen_program() -> None:
    subprocess.Popen([input(), "-v"])


def popen_keyword() -> None:
    subprocess.Popen(shell=True, args=input())


def run_shell() -> None:
    subprocess.run(input(), shell=True)


def drained(xs: list[int]) -> list[int]:
    return list(x for x in xs)


def drained_tuple(xs: list[int]) -> tuple[int, ...]:
    return tuple(x + 1 for x in xs)
