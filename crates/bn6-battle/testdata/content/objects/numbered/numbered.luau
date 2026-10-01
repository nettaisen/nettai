--!strict
-- A kind registered by number (the v1 form: a module that returns its
-- `update`), for the tests of registration by number. It ends at once.

local numbered = {}

function numbered.update(me: Object)
    me:free()
end

return numbered
