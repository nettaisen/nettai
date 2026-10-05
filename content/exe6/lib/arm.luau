--!strict
-- The arm a navi's chip attachment shows (`sub_80EBAE8`): the attachment's
-- animation (its second parameter) by who holds it. A virus has the plain
-- arm (0). A navi (actor type 1) has its AI index's, up to 0xC. A player
-- navi has its AI index's (0x13 shows 0xE), or, as MegaMan, one by form:
-- the plain arm for forms below 0xB, 0xD for form 0xC and forms from 0x12
-- (but 0x17), 0xC for the others.

local kind = require("../rules/forms/kind")

local arm = {}

-- `sub_80EBAE8`.
function arm.anim(navi: Object): number
    local t, ai = navi.actor_type, navi.ai_index
    if t == "navi" then
        return if ai < 0x0D then ai else 0
    elseif t == "player" then
        if ai ~= 0 then
            return if ai == 0x13 then 0x0E else ai
        end
        -- MegaMan's, by his form: 0, or a Beast form's (0xC for Gregar's,
        -- 0xD for Falzar's).
        if not kind.beast(navi.alliance) then
            return 0
        end
        return if battle.navi(navi.alliance).form.game == "gregar" then 0x0C else 0x0D
    end
    return 0
end

return arm
