--!strict
-- Roll's heart (attack object #0x4E, `sub_80CE118`): it sinks from 160
-- pixels above her user's panel, 3 pixels a tick, sparkling every 10
-- ticks; landed, it heals the user by its damage (or to full, with none),
-- with a sound and an effect, and goes 60 ticks later. See
-- docs/engine/chips.md §3.6.20.

local roll_heart = {}

local SPRITE = "0C-50"
local ANIM = 4

-- Its height, and how far it sinks a tick (Z's whole part).
local HEIGHT = 0xA0
local SINK = 3

-- The sparkle (effect 0x26), and the effect on the healed navi.
local SPARKLE, HEALED = 0x26, 6

-- `sub_80CE224`: a heart over (x, y) for `user`, healing `amount`;
-- `params` are the spawner's register (its owner's flags byte). (Its
-- position is the spawner's registers, all of which its init overwrites.)
function roll_heart.spawn(user: Object, x: number, y: number, amount: number, params: { number }): Object?
    local o = battle.spawn_kind("roll-heart", Vec3.zero, params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.damage = amount
        o.stamina = bit32.rshift(amount, 16)
        o.alliance = user.alliance
        o.related1 = user
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80CE198`: sink, sparkling every 10 ticks; landed, hidden, on to
-- the heal.
local function sink(me: Object)
    local t = me.timer + 1
    me.timer = t
    if t % 10 == 0 then
        battle.effect(battle.jitter(0x10, me.pos), SPARKLE)
    end
    local p = me.pos
    local z = bit32.rshift(bit32.band(p.z, 0xFFFF_0000), 16) - SINK
    if z >= 0 then
        me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), z * 0x10000))
        return
    end
    me.visible = false
    me:set_action(4)
end

-- `sub_80CE1D6`: heal the user (`object_addHP`), then 60 ticks.
local function heal(me: Object)
    if me.phase == 0 then
        local user = me.related1 :: Object
        local amount = me.damage
        if amount == 0 then
            amount = user.max_hp
        end
        user.hp = math.min(user.hp + amount, user.max_hp)
        battle.play_sound(0x8A)
        battle.effect(user.pos, HEALED)
        me.timer = 0x3C
        me.phase = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        return
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = sink, [4] = heal }

-- `sub_80CE178`.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("Roll's heart action %#x reads past its table (off_80CE190)", me.action))
    end
    action(me)
    me:update_sprite()
end

-- `sub_80CE138`: high over its panel; then its first update at once.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = ANIM
    me.anim_loaded = 0
    sprite:set_animation(ANIM)
    sprite:step()
    sprite.shadow = "with_sprite"
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, HEIGHT * 0x10000)
    me.timer = 0
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80CE118`.
function roll_heart.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return roll_heart
