--!strict
-- The Cross special's chips (BN6's beast system reads them: rules/beast/
-- cross-special.luau): the test content's one row, a chip it borrows.

local minibomb = require("../chips/minibomb/chip")

return define.rules("test:cross-special", {
    rows = {
        { minibomb },
    },
})
