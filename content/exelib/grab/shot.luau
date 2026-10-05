--!strict
-- A grab shot, as EXE5 and EXE6 share it (EXE6's attack object #0xF,
-- `sub_80C6414`; EXE5's #0xF): AreaGrab's and PanelGrab's. Each game makes
-- its own kind (`shot.make_kind`: its sprite, sounds and collision types).
-- It drops onto its panel from high up in 32 ticks, hits
-- whatever stands there, and takes the panel for its side if it is the
-- other side's, empty, and not the last of that side's full columns (the
-- panel comes back later with the stolen-area return). Its controller makes
-- it run while dimmed. See docs/engine/chips.md §3.6.8.

local panels = require("../panels")
local regions = require("../regions")

local shot = {}

-- A game's grab shot: its kind's key, its sprite, the sounds of its fall
-- and its landing, and the collision types of its hit (what it hits, and
-- what it is).
export type Look = {
    id: string,
    sprite: SpriteAsset,
    fall_sound: Sound,
    land_sound: Sound,
    target: CollisionType,
    self_type: CollisionType,
}
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

-- `sub_80C64A0`: fall; on landing, hit the panel and take it.
local function fall(me: Object, look: Look)
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
        hit_effect = nil,
        target = look.target,
        self_type = look.self_type,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 1,
    })
    if hit then
        hit.run_while_dimmed = true
    end
    local takes = TAKES[me.alliance]
    if field.check(x, y, takes.require, takes.forbid) and panels.can_take(x, y, bit32.bxor(me.alliance, 1)) then
        -- (The original takes the side from its first parameter, which its
        -- spawn sets to its side.)
        field.set_alliance(x, y, me.alliance)
        field.set_column_timer(x, STOLEN_TICKS)
    end
    me.action = 4
end

-- `sub_80C647C`: fall, land, then play out the landing.
local function tick(me: Object, look: Look)
    local a = me.action
    if a == 0 then
        fall(me, look)
    elseif a == 4 then
        -- sub_80C6524
        me.anim = 1
        battle.play_sound(look.land_sound)
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
local function init(me: Object, look: Look)
    me.visible = true
    me.sprite:load(look.sprite)
    me.sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0xFF
    me.vel = Vec3.new(me.vel.x, me.vel.y, FALL_SPEED)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(cx, cy, HEIGHT)
    battle.play_sound(look.fall_sound)
    me.lifecycle = "update"
    tick(me, look)
end

-- A game's grab shot kind.
function shot.make_kind(look: Look): Kind
    return define.kind {
        id = look.id,
        pool = "attack",
        -- `sub_80C6414`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
                return
            end
            me:update_sprite_while_dimmed()
        end,
    }
end

-- `sub_80C6548`: a grab shot (of `kind`, a game's) for `owner`'s side onto
-- (x, y), with the damage word `damage`. (Its position is the spawner's
-- registers until its init places it.)
function shot.spawn(kind: Kind, owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = owner.alliance
        o:set_damage_word(damage)
    end
    return o
end

return shot
