--!strict
-- BN6's form changes (docs/design/rules-in-luau.md §3.1): the five
-- sequences of `sub_8014A38` that change a navi into a Cross, Beast Out, a
-- Cross in Beast Out or Beast Over (change.luau; every BN6 form names the
-- action as its `change`), the Cross navi's image merging (merge.luau), and
-- what a change notes of the round (`sub_800AB2E`: Beast Out used, crossed;
-- read after the battle).
--
-- The turn-start sequencer that asks for the change, and the revert, are
-- the framework's (BN5 has them too).

local change = require("@self/change")

local forms = {}

export type State = {
    -- `byte_203EAE0` +2: the navi went Beast Out this battle (a navi that
    -- did not gets a turn back).
    beast_out_used: boolean,
    -- `byte_203EAE0` +0xB: the navi crossed this battle (the busting level).
    crossed: boolean,
}

-- The change, as the pause handler runs it.
forms.action = define.action {
    id = "forms/change",
    state = { timer = "u16" },
    update = change.update,
}

forms.system = define.system {
    id = "forms",
    state = { beast_out_used = "bool", crossed = "bool" },
    -- What BN6's forms say of themselves (docs/design/rules-in-luau.md §7.5;
    -- kind.luau reads the kind): their kind (every form but the base form
    -- names one), whose game's form each is (the Beast's roar, a Cross's
    -- game), the navi a Cross (and one in Beast Out) is made with, and the
    -- animation a change into a Cross lets the navi go of (`sub_8014B18`:
    -- GroundCross's drill).
    extends = {
        form = {
            kind = { "cross", "beast", "cross_beast", "beast_over" },
            game = { "gregar", "falzar" },
            cross_of = "navi",
            cross_release_anim = "u8",
        },
    },
    actions = { forms.action },
}

return forms
