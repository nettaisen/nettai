--!strict
-- EXE4's auto battle (docs/design/exe4-map.md §18 item 74): in a netbattle, the dark soul of a dark MegaMan (soul 15,
-- navis/megaman/forms/darksoul), whose idle the AI drives (./ai) on his own player's auto battle data (./block).

local block = require("@self/block")

local auto_battle = {}

-- A player's data as their setup states it and the send: the setup's and the state's types, the send as the round is
-- set up, and what `validate` says of the data.
auto_battle.block = block

-- What a side keeps of it through a round: its auto battle data as its console sent it as the battle started
-- (./block), which the AI plays and turns.
auto_battle.state = {
    auto_battle_places = schema.list(block.PLACE, block.PLACES),
} :: StateSchema

return auto_battle
