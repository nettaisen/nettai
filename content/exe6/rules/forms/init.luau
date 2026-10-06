--!strict
-- EXE6's form changes (docs/design/rules-in-luau.md §3.1): the five
-- sequences of `sub_8014A38` that change a navi into a Cross, Beast Out, a
-- Cross in Beast Out or Beast Over (change.luau; every EXE6 form names the
-- action as its `change`), the Cross navi's image merging (merge.luau), and
-- what a change notes of the round (`sub_800AB2E`: Beast Out used, crossed;
-- read after the battle).
--
-- The turn-start sequencer that asks for the change, and the revert, are
-- the framework's (EXE5 has them too).

local change = require("@self/change")

local forms = {}

export type State = {
    -- `byte_203EAE0` +2: the navi went Beast Out this battle (a navi that
    -- did not gets a turn back).
    went_beast_out: boolean,
    -- `byte_203EAE0` +0xB: the navi crossed this battle (the busting level).
    crossed: boolean,
}

-- The change, as the pause handler runs it.
forms.action = new.action {
    id = "forms/change",
    state = { timer = "u16" },
    update = change.update,
}

return forms
