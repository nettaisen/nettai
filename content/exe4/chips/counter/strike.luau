--!strict
-- The Counters' strike (attack object 0x17, 0x080CFD48: EXE5's 0x080DF93C's code but for its constants, which no
-- EXE5 chip uses): on the panel of a navi whose counter window is open, unturned, its sprite (14-10, its shadow drawn
-- with it) and sound (0xBE) and at once a hitbox on its panel 16 pixels up (an attack hitting navis, modifier 1, the
-- heavy spark: 0x080CFDB8's 0x04050601), then 38 ticks and gone.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local strike = {}

local SPRITE = asset.sprite("counter-strike")
local SOUND = asset.sound("counter-strike")
local Z = 0x10_0000
local HIT_MOD = 1
local TICKS = 0x25

-- 0x080CFDBC: 38 ticks, then gone.
local function tick(me: Object)
    if me.phase_init == 0 then
        me.timer = TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080CFD6C.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = Z,
        region = regions.single,
        hit_effect = sparks.heavy,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    me:set_lifecycle("update")
    tick(me)
end

strike.kind = new.kind {
    id = "counter/strike",
    pool = "attack",
    -- 0x080CFD48: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080CFDE4: a strike on (x, y) with the element and damage word, of `owner`'s side. (Its position is the registers
-- until its init places it.)
function strike.spawn(owner: Object, x: number, y: number, element: number, damage: number)
    local o = battle.spawn(strike.kind, Vec3.new(y, element, 0))
    if o then
        o.element = element
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(damage)
        o.alliance = owner.alliance
    end
end

-- BattleState +0x44's list by its index (0x080CFE20): the local console's navi, the other's, and none past them in a
-- netbattle.
local function listed(i: number): Object?
    local here = battle.local_side()
    if i == 0 then
        return battle.player(here)
    elseif i == 1 then
        return battle.player(bit32.bxor(here, 1))
    end
    return nil
end

-- 0x080CFE02, the spawners' variant 0: from the other side's place in the local console's list (BattleState +0x44, the
-- local side +0x0D), three entries for a user of side 0 and one for side 1, a strike on each navi whose counter window
-- is open. (So on side 1's console a side-0 user's entries are side 1's navi, its own and none. A navi always has its
-- collision, whose absence would end the search.)
function strike.spawner(user: Object, element: number, damage: number)
    local count = if user.alliance == 0 then 3 else 1
    local i = bit32.bxor(bit32.bxor(user.alliance, 1), battle.local_side())
    for _ = 1, count do
        local o = listed(i)
        if o and o.collision.counter_timer ~= 0 then
            strike.spawn(user, o.panel_x, o.panel_y, element, damage)
        end
        i += 1
    end
end

return strike
