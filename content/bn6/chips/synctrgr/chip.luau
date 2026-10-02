--!strict
-- SyncTrgr: the navi's mood goes to the top, Full Synchro, unless its
-- emotion is held (instant chip effect 13, `sub_80EC44C`, run once by
-- action 0x1C). (Its record's menu classification bits, 0x10 and 0x20,
-- which no battle routine reads, stay out: §3.1.)

local SOUND = asset.sound("cross-change")

-- The mood Full Synchro starts from.
local FULL_SYNCHRO = 0xFF

-- `sub_80EC44C`.
local function sync(user: Object, _spec: InstantChipSpec)
    battle.play_sound(SOUND)
    -- sub_8015BEC
    battle.set_mood(user.alliance, FULL_SYNCHRO)
end

return define.chip {
    id = "synctrgr",
    codes = { "A", "B", "C", "*" },
    element = "null", family = "null", class = "mega",
    rarity = 4, mb = 80, damage = 0, hit_param = 10, lockout = 20,
    library = { number = 0, index = 65, sort = 0 },
    slot_in_limit = 1,
    instant = sync,
}
