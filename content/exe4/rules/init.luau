--!strict
-- EXE4's rules, their one definition (docs/design/rules-in-luau.md §2.5). EXE4's port is under way
-- (docs/design/exe4-map.md §13): this pack holds its compat tables (for the extractor) and no rules yet, so EXE4
-- doesn't load as a game. The sections every game's rules have (`Rules`: link_pick, flow, panels, reactions, pools,
-- effects, status, chip_use, fresh_stats) come with the port's next step, each from EXE4's code, and with them this
-- table's `Rules` type.

local rules = {}

return rules
