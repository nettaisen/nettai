--!strict
-- The Cross special's chips (EXE6's rules/beast reads them: rules/beast/
-- cross-special.luau): the test content's one row, a chip it borrows.

local mini_bomb = require("../chips/mini_bomb").mini_bomb

local CROSS_SPECIAL = schema.record_type("cross_special")

return new.record(CROSS_SPECIAL, {
    id = "cross_special",
    rows = {
        { mini_bomb },
    },
})
