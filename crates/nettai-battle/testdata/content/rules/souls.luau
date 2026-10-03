--!strict
-- A mix with a section of its own (docs/design/rules-in-luau.md P1 item 8):
-- the test content's stock rules, its sides' HP bug periods its own.

local systems = require("./systems")

local status = define.rules("test:souls/status", { hp_bug_periods = { 0, 1, 2, 3, 4, 5, 6, 7 } })

return define.ruleset { id = "test:test-souls", base = systems.stock, sections = { status } }
