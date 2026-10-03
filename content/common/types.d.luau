-- The shared types of content/common's modules (docs/design/rules-in-luau.md): what a game passes them (its looks,
-- kinds and constants) and what they hand back. Every content folder's definitions check together, so BN5's and
-- BN6's modules use these as their own.

-- A trap chip's object (a "trap" record, @common/traps/controller): `set` spawns it for the controller and its user,
-- with the chip and bonus the controller keeps.
export type Trap = {
    set: (controller: Object, user: Object, chip: Chip?, bonus: number) -> Object?,
}
