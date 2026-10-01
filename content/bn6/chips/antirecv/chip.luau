--!strict
-- AntiRecv: a hidden trap that turns the other side's next recovery into
-- damage (`sub_800E2FC` and Roll's `sub_80E192C`, with
-- objects/anti-recovery). Its controller is the trap chips'
-- (lib/traps/controller); it sets no object of its own.
--
-- Its record stays the pack's for now (the recoveries name it by number):
-- this is its `dimming` hook, which registration by number runs
-- (chips/0ba-antinavi).

local traps = require("../../lib/traps/controller")

return { dimming = traps.hook() }
