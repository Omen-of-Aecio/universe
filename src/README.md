# Project setup #

*mediators/:*
	Our "top-level" business-logic. Just functions, no data structures (important!)

	Depends on mediators/, glocals/, libs/ and other crates

*glocals/:*
	Just dumb data structures

	Depends on glocals/, libs/ and other crates

*libs/:*
	Data structures + impls

	Depends only on libs/ and other crates

## This ensures ##

_EASY_ access to any data in our top-level (mediators). Since we just have a tree of data (read: nested structs), we can iterate rapidly in the top-level logic.

_SEPARATION_ of business logic and smaller domains in libs/. libs/ doesn't care about our business logic, much like std::string::String does not care.

## Glocals ##

Glocals _MUST_ be passed around. They are called "glocals" because they fill the role of "globals" but are "local", hence "glocal".
