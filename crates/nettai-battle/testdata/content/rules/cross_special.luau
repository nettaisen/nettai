--!strict
-- The Cross special's chips (EXE6's rules/beast reads them: rules/beast/
-- cross-special.luau): the test content's one row, a chip it borrows.

local minibomb = require("../chips/minibomb").minibomb

local CROSS_SPECIAL = schema.record_type("cross-special")

return new.record(CROSS_SPECIAL, {
    id = "cross-special",
    rows = {
        { minibomb },
    },
})
