--!strict
-- A chip series composing the family's action with its own length (the
-- pattern that replaces a table indexed by subtype), and a chip whose
-- action starts another when it ends.

local ticker = require("../lib/ticker")

local chips: { [string]: Chip } = {
    ["test/ticker1"] = { action = ticker.action { length = 6 } },
    ["test/ticker2"] = { action = ticker.action { length = 9 } },
    ["test/ticker3"] = { action = ticker.action { length = 3, next = ticker.action { length = 2 } } },
}

return chips
