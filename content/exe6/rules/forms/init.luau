--!strict
-- EXE6's form changes (docs/design/rules-in-luau.md §3.1): the five
-- sequences of `sub_8014A38` that change a navi into a Cross, Beast Out, a
-- Cross in Beast Out or Beast Over (change.luau; every EXE6 form names the
-- action as its `change`), the Cross navi's image merging (merge.luau), and
-- what a change notes of the round, which nettai keeps none of: what the
-- original's `sub_800AB2E` notes in `byte_203EAE0` it reads only after the
-- battle. Its +2, the navi went Beast Out, gives the save's Beast Out
-- counter (NaviStats +0x21) a turn back, to 3 at most, when it didn't
-- (0x08007DB4, as the battle ends): the next battle's, which nettai's sets
-- don't carry (each round starts from the first's stats: nettai-match's
-- `next_round`). Its +0xB, the navi crossed, the busting level reads, the
-- results screen's. A second Beast Out in a battle is refused by the
-- counter and the screen's own record (`sub_802A57E`: tired, Beast Over
-- instead, ./beast), which nettai has.
--
-- The turn-start sequencer that asks for the change, and the revert, are
-- the framework's (EXE5 has them too).

local change = require("@self/change")

local forms = {}


-- The change, as the pause handler runs it.
forms.action = new.action {
    id = "forms/change",
    state = { timer = "u16" },
    update = change.update,
}

return forms
