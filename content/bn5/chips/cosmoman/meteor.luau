--!strict
-- CosmoMan's meteor (BN5's attack object #0xBF, 0x080DE708; BN6 keeps its code, `sub_80DCEF4`, which no BN6 chip
-- spawns): it falls from 72 pixels up over its caller's column onto its target panel in 10 ticks (starting a panel
-- further toward the enemy when the target is in that column, so it never falls straight), then hits the panel
-- (a single panel, running while dimmed) with an explosion; a panel that isn't solid or held swallows it. It is
-- CosmoMan's meteor, or a field object he took (its look, `sub_800F26C`).

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local regions = require("@exelib/regions")

local meteor = {}

export type State = {
    -- The field object it is (its look), or none for CosmoMan's own meteor (+0x60).
    look: Identity?,
}

local SPRITE = asset.sprite("bn5:cosmoman")
local SOUND_FALL = asset.sound("bn5:justice-one")
local SOUND_HIT = asset.sound("bn5:hit-bomb-1")
-- The explosion where it lands (effect 0).
local EXPLOSION = define.effect { sprite = asset.sprite("bn5:explosion"), anim = 0, palette = 0 }
-- What a panel must hold to take it (solid, or something on it).
local TAKES = 0x0F80_0010

-- 0x080DE72C: its look; over its caller's column on its target's row, 72 pixels up, heading for the target.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    local look = s.look
    local worn = look ~= nil and me:wear_junk_look(look)
    if not worn then
        -- CosmoMan's meteor (category 8 0x10, animation 10, palette 0), drawn with its sprite.
        sprite:load(SPRITE)
        me.visible = true
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 10, 10
        sprite:set_animation(10)
        sprite.palette = 0
    end
    local caller = me.related1 :: Object
    me.panel_x, me.panel_y = caller.panel_x, me.future_panel_y
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0x48_0000)
    local dir = 1 - 2 * me.alliance
    local dx = me.panel_x - me.future_panel_x
    if dir >= 0 then
        dx = -dx
    end
    local vx = int.mul32(dir * dx, 0x4_0000)
    if me.panel_x == me.future_panel_x then
        local q = me.pos
        me.pos = Vec3.new(int.i32(q.x + dir * 0x28_0000), q.y, q.z)
        vx = -dir * 0x4_0000
    end
    me.vel = Vec3.new(vx, me.vel.y, 0x7_0000)
    if look == nil then
        sprite:set_flip(if vx < 0 then 1 else 0)
    elseif not worn then
        -- (A look that shows nothing: CosmoMan's meteor, flipped by its side.)
        sprite:set_flip(me.alliance)
    end
    me:set_lifecycle("update")
end

-- Action 0 (0x080DE840): 10 ticks falling; then onto its target, or into the hole there.
local function fall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_FALL)
        me.timer = 0xA
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z - v.z))
        return
    end
    if bit32.band(field.flags(me.future_panel_x, me.future_panel_y), TAKES) ~= 0 then
        me:set_action(4)
    else
        me:set_lifecycle("destroy")
    end
end

-- Action 4 (0x080DE894): the hit on its target (running while dimmed), the explosion; it goes.
local function land(me: Object)
    local x, y = me.future_panel_x, me.future_panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.impact,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), EXPLOSION, 0, 0, 0)
    me.visible = false
    battle.play_sound(SOUND_HIT)
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = fall, [4] = land }

meteor.kind = define.kind {
    id = "bn5:cosmoman/meteor",
    pool = "attack",
    state = { look = "identity" },
    -- 0x080DE708.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            -- 0x080DE816.
            if battle.over() then
                me:set_lifecycle("destroy")
            else
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("CosmoMan's meteor action %#x reads past its table (0x080DE838)", me.action))
                end
                action(me)
            end
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080DE8D4: a meteor of `caller` onto (x, y), with its element, damage word and look (none: CosmoMan's own).
function meteor.spawn(caller: Object, x: number, y: number, element: number, damage: number, look: Identity?): Object?
    local o = battle.spawn(meteor.kind, Vec3.zero)
    if o then
        o.future_panel_x, o.future_panel_y = x, y
        o.element = element
        o.related1 = caller
        o:set_damage_word(damage)
        ;(o.state :: State).look = look
        o.alliance, o.flip = caller.alliance, caller.flip
        o.run_while_dimmed = true
    end
    return o
end

return meteor
