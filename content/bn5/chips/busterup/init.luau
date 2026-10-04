--!strict
-- BusterUp (BN5 chip 0x073): action 0x1A's effect 10 (0x0800E5B2, BN6's `sub_8010820`): the buster's attack level
-- up by one (to 9 at most once it reaches 10), as BN6's. BN5 shows the heal effect (effect 6, flipped) on the user's
-- panel where BN6 shows its own sparkle at an attach point, and in ColonelSoul the navi gets AirShoes.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local effects = require("../lib/effects")
local souls = require("../navis/megaman/souls")

local SOUND = asset.sound("hub")

local function raise(user: Object, _spec: InstantChipSpec)
    local stats = battle.navi(user.alliance)
    local level = stats.attack + 1
    if level >= 10 then
        level = 9
    end
    stats.attack = level
    local x, y = field.center(user.panel_x, user.panel_y)
    battle.effect(Vec3.new(x, y, 0), effects.heal, 1)
    battle.play_sound(SOUND)
    if souls.of(user) == "colonel" then
        user:set_status("air_shoe", true)
    end
end

return define.chip {
    id = "busterup",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 5,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    instant = raise,
}
