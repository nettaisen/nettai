--!strict
-- ColonelSoul's soldiers' damage words (0x08011C50), which its status reset arms its side with (./init) and BusterUp
-- rewrites in it (chips/busterup).

local soldiers = {}

-- 0x08011C50: the words by the buster's Attack (0x0800FE5E: plus one): 40 and 10 more a point for the sword's soldier,
-- 10 and 2 more for the gun's, each with counter byte 0x94 and the paralyzing flag 0x4000.
function soldiers.words(navi: Object): (number, number)
    local level = battle.navi(navi.alliance).attack + 1
    return 40 + 10 * level + 0x0094_4000, 10 + 2 * level + 0x0094_4000
end

-- 0x08011C50 from BusterUp (0x0800E5FA): the words by the Attack now, the side's arming left as it is (0x080CABF8
-- alone).
function soldiers.reword(navi: Object)
    local melee, ranged = soldiers.words(navi)
    obstacle.set_conversion_words(navi.alliance, melee, ranged)
end

return soldiers
