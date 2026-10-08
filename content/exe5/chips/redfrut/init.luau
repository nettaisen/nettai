--!strict
-- RedFrut1, RedFrut2 and RedFrut3: the redfrut series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local fruit = require("@self/fruit")
local operation_battle = require("../lib/operation_battle")

-- The gifts: their amounts.
local HEAL, MOOD, INVULNERABLE_TICKS = 300, 50, 0x1A4

-- 0x080D8118: RedFrut1's: 300 HP back (AntiRecv turns it), and the mood up 50 (0x080127E8: to 254 at most, a mood
-- of 0 left as it is).
local HEAL_GIFT = fruit.gift(function(navi: Object)
    navi:heal(HEAL, true)
    local stats = battle.navi(navi.alliance)
    local mood = stats.mood
    if mood ~= 0 then
        stats.mood = math.min(mood + MOOD, 0xFE)
    end
end)

-- 0x080D8130: RedFrut2's: invulnerable for 420 ticks (`object_setInvulnerableTime`).
local SHIELD_GIFT = fruit.gift(function(navi: Object)
    navi:set_status_timer("invulnerable", INVULNERABLE_TICKS)
    navi:set_status("invulnerable", true)
end)

-- 0x080D813C: RedFrut3's: a player's navi, out of the operation battle, whose emotion (0x0801270C) is one of the
-- first four, to Full Synchro (0x080127D6: a mood of 0 left as it is).
local SYNCHRO_GIFT = fruit.gift(function(navi: Object)
    if navi.actor_type ~= "player" or operation_battle() then
        return
    end
    local stats = battle.navi(navi.alliance)
    -- EXE5's emotion (exe5-map.md §15.10): in battle mode 1 Full Synchro (2) or normal (0); else a soul's 4 first (the
    -- engine's emotion says normal for it), then anger's 3, a mood of 0's 5 (worn out), and 0 to 2 by the mood.
    if battle.mode() ~= 1 and not stats.form.base then
        return
    end
    if battle.emotion(navi.alliance) == "worn_out" then
        return
    end
    if stats.mood ~= 0 then
        stats.mood = 0xFF
    end
end)

-- RedFrut1 (EXE5 chip 0x042): a hopping fruit whose breaker's side gets 300 HP back and 50 more mood (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 0, five hops).
local redfrut1: Chip = {
    codes = { "E", "K", "Z" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 24,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    instant = fruit.instant { gift = HEAL_GIFT, palette = 0, hops = 5 },
    auto_battle = { class = "at_once" },
}

-- RedFrut2 (EXE5 chip 0x043): a hopping fruit whose breaker's side gets invulnerability for 420 ticks (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 1, five hops).
local redfrut2: Chip = {
    codes = { "F", "T", "W" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 48,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    instant = fruit.instant { gift = SHIELD_GIFT, palette = 1, hops = 5 },
    auto_battle = { class = "at_once" },
}

-- RedFrut3 (EXE5 chip 0x044): a hopping fruit whose breaker's side gets Full Synchro (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 2, five hops).
local redfrut3: Chip = {
    codes = { "H", "O", "U" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 62,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    instant = fruit.instant { gift = SYNCHRO_GIFT, palette = 2, hops = 5 },
    auto_battle = { class = "at_once" },
}

return { redfrut1 = redfrut1, redfrut2 = redfrut2, redfrut3 = redfrut3 }
