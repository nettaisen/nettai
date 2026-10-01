--!strict
-- A grab shot (the original's attack object #0xF, `sub_80C6414`): AreaGrab's
-- and PanelGrab's. It drops onto its panel from high up in 32 ticks, hits
-- whatever stands there, and takes the panel for its side if it is the
-- other side's, empty, and not the last of that side's full columns (the
-- panel comes back later with the stolen-area return). Its controller makes
-- it run while dimmed. See docs/engine/chips.md §3.6.8.

local collision = require("../../rules/collision")
local regions = require("../regions")

local shot = {}

local SPRITE = asset.sprite("grab-shot")
local SOUND_FALL, SOUND_LAND = asset.sound("grab-shot-2"), asset.sound("grab-shot")
-- How high it starts, and how fast it falls, 16.16.
local HEIGHT, FALL_SPEED = 0x100_0000, 0x8_0000
-- A stolen panel stays the thief's this long
-- (`object_setPanelAllianceTimerLong`).
local STOLEN_TICKS = 0x708

-- The panels it takes: the other side's, with nothing on them
-- (`byte_80C6514`, by its side): the owner bit (0x20) and everything that
-- occupies a panel (bodies, neutral objects, blockers, reservations).
local ALLIANCE_1, OCCUPIED = 0x20, 0x0F880080
local TAKES = {
    [0] = { require = ALLIANCE_1, forbid = OCCUPIED },
    [1] = { require = 0, forbid = bit32.bor(OCCUPIED, ALLIANCE_1) },
}

-- `sub_800D618`: (x, y) is `victim`'s home but someone else holds it.
local function taken_from(x: number, y: number, victim: number): boolean
    local p = field.panel(x, y)
    return p ~= nil and p.home ~= p.alliance and p.home == victim
end

-- `sub_800D6CC`: column x is `victim`'s home, and all its.
local function full_home_column(x: number, victim: number): boolean
    if field.column(x).home ~= victim then
        return false
    end
    for y = 1, 3 do
        local p = field.panel(x, y)
        if not p or p.alliance ~= victim then
            return false
        end
    end
    return true
end

-- `sub_800D668`: `victim` may lose (x, y): its column isn't `victim`'s
-- home, or `victim` keeps another full home column on either side before
-- a panel it already lost.
local function can_take(x: number, y: number, victim: number): boolean
    if field.column(x).home ~= victim then
        return true
    end
    for _, dir in { 1, -1 } do
        local c = x + dir
        while field.valid(c, y) and not taken_from(c, y, victim) do
            if full_home_column(c, victim) then
                return true
            end
            c += dir
        end
    end
    return false
end

-- `sub_80C64A0`: fall; on landing, hit the panel and take it.
local function fall(me: Object)
    local p = me.pos
    local z = int.i32(p.z - me.vel.z)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    me.pos = Vec3.new(p.x, p.y, 0)
    local x, y = me.panel_x, me.panel_y
    local hit = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        region = regions.single,
        -- None.
        hit_effect = 0xFF,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 1,
    })
    if hit then
        hit.run_while_dimmed = true
    end
    local takes = TAKES[me.alliance]
    if field.check(x, y, takes.require, takes.forbid) and can_take(x, y, bit32.bxor(me.alliance, 1)) then
        -- (The original takes the side from its first parameter, which its
        -- spawn sets to its side.)
        field.set_alliance(x, y, me.alliance)
        field.set_column_timer(x, STOLEN_TICKS)
    end
    me.action = 4
end

-- `sub_80C647C`: fall, land, then play out the landing.
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        fall(me)
    elseif a == 4 then
        -- sub_80C6524
        me.anim = 1
        battle.play_sound(SOUND_LAND)
        me.action = 8
    elseif a == 8 then
        -- sub_80C6536: gone once the landing plays out (the action stays).
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.lifecycle = "destroy"
        end
    else
        error(string.format("grab shot action %#x reads past its table", a))
    end
    me:update_sprite()
end

-- `sub_80C6438`: high above its panel.
local function init(me: Object)
    me.visible = true
    me.sprite:load(SPRITE)
    me.sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0xFF
    me.vel = Vec3.new(me.vel.x, me.vel.y, FALL_SPEED)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(cx, cy, HEIGHT)
    battle.play_sound(SOUND_FALL)
    me.lifecycle = "update"
    tick(me)
end

shot.kind = define.kind {
    id = "grab/shot",
    pool = "attack",
    -- `sub_80C6414`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80C6548`: a grab shot for `owner`'s side onto (x, y), with the
-- damage word `damage`. (Its position is the spawner's registers until its
-- init places it.)
function shot.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(shot.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = owner.alliance
        o:set_damage_word(damage)
    end
    return o
end

return shot
