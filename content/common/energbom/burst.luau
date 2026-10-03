--!strict
-- The energy burst, as BN5 and BN6 share it (BN6's attack object #0x11,
-- `sub_80C67F8`; BN5's #0x11): each game makes its own (`burst.make`: its
-- kind, sprite, sound and collision types):
-- what EnergBom's and MegEnBom's bomb leaves where it lands. It hits its
-- panel three times, 7 ticks apart (the first at once), with hit
-- modifiers 1, 1 and 3, then shows for 19 more ticks.

local regions = require("../regions")

local burst = {}

-- A game's burst: its kind's key, sprite and sound, and its hits' collision
-- types (what it hits, and what it is).
export type Look = { id: string, sprite: SpriteAsset, sound: Sound, target: CollisionType, self_type: CollisionType }

local HIT_INTERVAL, HITS, LINGER = 7, 3, 0x14
-- The hits' hit modifiers, by hit (`dword_80C68AC`).
local HIT_MODS = { [0] = 1, [1] = 1, [2] = 3 }

-- `sub_80C6854`: the hits, then the linger.
local function tick(me: Object, look: Look)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            return
        end
        me.timer = HIT_INTERVAL
        local n = me.timer2
        local hit_mod = HIT_MODS[n]
        if hit_mod == nil then
            error(string.format("energy burst hit %d reads past dword_80C68AC", n))
        end
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = nil,
            target = look.target,
            self_type = look.self_type,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = hit_mod,
        })
        n += 1
        me.timer2 = n
        if n < HITS then
            return
        end
        me.phase, me.phase_init = 4, 0
        me.timer = LINGER
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C681C`.
local function init(me: Object, look: Look)
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0xFF
    me:set_coordinates_from_panel()
    battle.play_sound(look.sound)
    me:set_lifecycle("update")
    tick(me, look)
end

-- `sub_80C68B0`: what a landed bomb leaves (its variant's `after`): a burst
-- (of `kind`) on its panel, of its side, element and damage word. (Its
-- position is the spawner's registers, the panel row and element, until its
-- init places it.)
local function leave(kind: Kind, bomb: Object)
    local x, y = bomb.panel_x, bomb.panel_y
    local o = battle.spawn(kind, Vec3.new(y, bomb.element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = bomb.element
        o.alliance = bomb.alliance
        o.damage = bomb.damage
        o.stamina = bomb.stamina
        o.related1 = bomb
    end
end

-- A game's burst: its kind, and what a landed bomb leaves.
function burst.make(look: Look): { kind: Kind, leave: (bomb: Object) -> () }
    local kind = define.kind {
        id = look.id,
        pool = "attack",
        -- `sub_80C67F8`.
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
            me:update_sprite()
        end,
    }
    return {
        kind = kind,
        leave = function(bomb: Object)
            leave(kind, bomb)
        end,
    }
end

return burst
