--!strict
-- MagnetMan's Mag-Barr (EXE5 DS's party battle: 0x02098AE6, in the navi's low-HP check 0x02098A84 after the NaviCust's
-- Tango; his controller the DS's effect object 0x91, 0x02225D2C, his actor its actor object 0x32, 0x021FF6E0;
-- docs/design/exe5ds-map.md §4.7): once a battle for each side (battle flags 0x200 and 0x400), the navi idle at a
-- quarter of its HP or less (and below its maximum) with no barrier, the light test passed and MagnetMan a member who
-- may support (not on the field), the battle dims (the other side can't cut in), Mag-Barr's telop shows, and MagnetMan
-- comes behind the navi, raises his magnet arm, gives the navi a barrier (10 HP, at every level: 0x0222722C) and
-- immobilizes every enemy body from his column forward.

local attachment = require("../../objects/attachment")
local barriers = require("../../lib/barriers")
local chips_mag_barr = require("../../chips/mag_barr")
local collision = require("../../rules/collision")
local immobilizer = require("@exelib/instant/immobilizer")
local magnetman = require("../magnetman").magnetman
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")
local status = require("../../rules/status")

local support = {}

local MAG_BARR = chips_mag_barr["mag-barr"]
-- Each side's once-a-battle mark (battle flags 0x200, 0x400).
local USED = { [0] = 0x200, [1] = 0x400 }
local APPEAR_SOUND = asset.sound("appear")
-- His magnet arm (the DS's attachment row 0x3A: 10-3C, its animation 0), the ticks he holds it, when the barrier comes
-- and when the enemies are held.
local MAGNET_ARM = attachment.look { sprite = asset.sprite("magnet-arm"), palette = 0 }
local ANIM_MAGNET, MAGNET_ENDS, BARRIER_AT, HOLD_AT = 0x10, 0x5A, 0x1E, 0x3C
-- The enemies' bodies' panel flag, by side (0x020CC7D4).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
-- The DS's immobilizer (T3 0x3F, the GBA's code but that its linger is its Param1, 20 here, and its Param2 1 plays
-- 0xFD): MagnetSoul's mark, its guard-breaking hit against navis that immobilizes (status 0x42).
local IMMOBILIZER = immobilizer.make_kind("magnetman/immobilizer", {
    sprite = asset.sprite("immobilized"),
    anim = 1,
    lasts = 0x14,
    sound = asset.sound("bass-anly"),
    status = status.immobilize_120,
    target = collision.hits_navis,
    self_type = collision.guard_breaking,
})

local LEAD_IN, APPEAR, READY, MAGNET, STAND, WARP, LEAD_OUT = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18

type ActorState = { controller: Object?, held: Object? }

local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    return t <= 0
end

local function go(me: Object, action: number)
    me.action, me.phase, me.phase_init = action, 0, 0
end

-- 0x02007BA8: an immobilizer (running while dimmed, 0x0220AF44) on every enemy body from his column forward, row by
-- row.
local function hold_enemies(me: Object)
    local enemy = ENEMY_BODY[me.alliance]
    for y = 1, 3 do
        local x = me.panel_x
        while field.valid(x, y) do
            if bit32.btest(field.flags(x, y), enemy) then
                local o = immobilizer.spawn(IMMOBILIZER, me, x, y)
                if o then
                    o.run_while_dimmed = true
                end
            end
            x += me.facing
        end
    end
end

-- 0x021FF7ED: the magnet arm up (animation 0x10); on its 31st tick the navi's barrier (its old barrier's visual ending,
-- a new one coming), on its 61st the enemies held; 91 ticks.
local function magnet(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_MAGNET
        attachment.attach(me, MAGNET_ARM, "held", { anim = 0, while_dimmed = true })
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == BARRIER_AT then
        barriers.raise(me.related1 :: Object, barriers.barrier_10)
    end
    if me.timer == HOLD_AT then
        hold_enemies(me)
    end
    me.timer += 1
    if me.timer > MAGNET_ENDS then
        go(me, STAND)
    end
end

local function update(me: Object)
    local a = me.action
    if a == LEAD_IN then
        if countdown(me, 4) then
            go(me, APPEAR)
        end
    elseif a == APPEAR then
        if me.phase_init == 0 then
            me.visible = true
            me.anim = 3
            battle.play_sound(APPEAR_SOUND)
        end
        if countdown(me, 4) then
            go(me, READY)
        end
    elseif a == READY then
        if me.phase_init == 0 then
            me.anim = 0
        end
        if countdown(me, 0x10) then
            go(me, MAGNET)
        end
    elseif a == MAGNET then
        magnet(me)
    elseif a == STAND then
        if me.phase_init == 0 then
            me.anim = 0
            -- (0x021FF878: the arm put away.)
            local held = (me.state :: ActorState).held
            if held then
                held:set_lifecycle("destroy")
            end
        end
        if countdown(me, 0x14) then
            go(me, WARP)
        end
    elseif a == WARP then
        if me.phase == 0 then
            me.anim, me.anim_loaded = 4, 0
            me.timer = 4
            me.phase = 4
        end
        me.timer -= 1
        if me.timer <= 0 then
            me.visible = false
            go(me, LEAD_OUT)
        end
    elseif a == LEAD_OUT then
        if countdown(me, 4) then
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("Mag-Barr's MagnetMan: action %#x reads past its table (0x02227244)", a))
    end
    me:update_sprite()
end

support.actor = new.kind {
    id = "magnetman/support",
    pool = "actor",
    state = { controller = "object", held = "object" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x021FF6F0: on the navi's panel, 10 pixels behind it and 10 up, MagnetMan's sprite, his shadow.
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x - me.facing * 0xA_0000, p.y - 0xA_0000, 0)
            local sprite = me.sprite
            sprite:load(magnetman.sprite)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_lifecycle("update")
            update(me)
        elseif l == "update" then
            update(me)
        else
            -- 0x021FF8F8: his controller's word for him cleared.
            support.controller.done((me.state :: ActorState).controller)
            me:free()
        end
    end,
}

-- His controller (the DS's effect object 0x91): the dim, Mag-Barr's telop, MagnetMan (0x02225D4C), the undim.
support.controller = party_support.controller {
    id = "magnetman/support-controller",
    bring = function(me: Object): Object?
        local navi = me.related1 :: Object
        local o = battle.spawn(support.actor)
        if o then
            o.panel_x, o.panel_y = me.panel_x, me.panel_y
            o.alliance, o.flip = navi.alliance, navi.flip
            o.element = me.element
            o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
            o.related1 = navi;
            (o.state :: ActorState).controller = me
        end
        return o
    end,
}

-- 0x02098AE6 (the rules' `low_hp`): true when MagnetMan comes (no damage of his own).
function support.low_hp(side: number, navi: Object): boolean
    if not party.light(side) or not party.may_support(side, magnetman) then
        return false
    end
    if battle.navi(side).navi == magnetman or navi.collision.barrier ~= "none" or navi.hp >= navi.max_hp then
        return false
    end
    if battle.rules_flag(USED[side]) then
        return false
    end
    battle.set_rules_flag(USED[side])
    support.controller.send(navi, 0, MAG_BARR)
    return true
end

return support
