--!strict
-- MagnetSoul's B+Back (`magnetsoul/back`): the original's weapon routine 0x25 (0x0800F9A8) sets the instant chips'
-- effect 10, no damage and a 120-tick wait before the next B+Back, and starts the instant chips' action (EXE5's 0x1A).
-- Effect 10 (0x080124D0) does nothing outside MagnetSoul; in it MegaMan glows (effect 0x26: sprite 14-1B on him for
-- 20 ticks) and every panel from his forward to the field's edge with an enemy body on it gets an immobilizing hit
-- (EXE5's attack object 0x3F, EXE6's code: @exelib/instant/immobilizer, with the immobilized mark).

local collision = require("../../../../rules/collision")
local glow = require("../../weapons/glow")
local immobilizer = require("@exelib/instant/immobilizer")
local status = require("../../../../rules/status")

local back = {}

-- EXE5's immobilizer: the mark (10-2D, its animation 1), a guard-breaking attack against navis that immobilizes for
-- 120 ticks (status 0x42), its sound.
back.immobilizer = immobilizer.make_kind("immobilizer", {
    sprite = asset.sprite("immobilized"),
    anim = 1,
    sound = asset.sound("immobilizer"),
    status = status.immobilize_120,
    target = collision.hits_navis,
    self_type = collision.guard_breaking,
})

-- The glow (effect 0x26's parameters 0x141B: 14-1B, animation 0, no shadow).
local GLOW = asset.sprite("magnetsoul-glow")
-- MagnetSoul, by its form's id.
local MAGNET_SOUL = "magnetsoul"
-- The wait before the next B+Back (the attack's lockout).
local COOLDOWN = 120

-- 0x080124D0: in MagnetSoul, the glow and the immobilizers ahead.
local function effect(user: Object, spec: InstantChipSpec)
    if battle.navi(user.alliance).form.id ~= MAGNET_SOUL then
        return
    end
    glow.spawn(user, GLOW)
    immobilizer.ahead(back.immobilizer, user, spec.panel_x, spec.panel_y)
end

-- 0x0800F9A8
local function setup(navi: Object)
    navi.attack_lockout = COOLDOWN
    navi.attack_damage = 0
    navi.hit_param = 0
    return nil
end

back.weapon = define.weapon {
    -- (A B+Back, which nothing charges: the charge table's row 0x25 (0x0801CBDE), Charge 5 reading on into row
    -- 0x26's first entry.)
    charge_ticks = { 60, 60, 60, 60, 60, 70 },
    setup = setup,
    instant = effect,
}

return back
