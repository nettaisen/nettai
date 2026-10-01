--!strict
-- BodyGrd, the Program Advance of AntiNavi, AntiSwrd and AntiDmg: a hidden
-- trap that swallows the next hit of 10 damage or more and answers with
-- its counter (the damage intake's `sub_802CEF4`; the counter is the role
-- `actions.body_guard_counter`). Its controller is the trap chips'
-- (lib/traps/controller); it sets no object of its own.
--
-- Its record stays the pack's for now (it is a Program Advance, and the
-- damage intake names it by number): this is its `dimming` hook, which
-- registration by number runs (chips/0ba-antinavi).

local traps = require("../../lib/traps/controller")

return { dimming = traps.hook() }
