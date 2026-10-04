--!strict
-- A mix (docs/design/rules-in-luau.md §2.2): the test content's stock rules
-- without BN6's forms system, with the marker after them.

local systems = require("./systems")
local forms = require("./forms")

return define.ruleset { id = "test-mix", base = systems.stock, remove = { forms.system }, add = { systems.marker } }
