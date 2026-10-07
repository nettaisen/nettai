--!strict
-- EXE4's Double Soul (docs/design/exe4-map.md §6), as EXE5's rules/souls is its Soul Unison: what the rules list of
-- it. So far its part of the custom screen (./custom: the hand's size); the souls themselves, their change, turns and
-- revert are still to port (exe4-map.md §18 item 25).

local custom = require("@self/custom")

local souls = {}

souls.custom = custom

return souls
