--!strict
-- EXE5's Voltz1-3 (action 0x1A's instant effect 16, 0x080D88CE): EXE5's own. A sensor (attack object #0x90,
-- 0x080D86E8; NameID 0xE4) drops from 160 pixels onto the panel three ahead of the user, 4 pixels a tick, its
-- collision on (and the panel reserved) for its last 16 pixels; landed, it waits 10 ticks, then shocks (its second
-- animation, the thunder ball's sound): a shock (attack object #0x92, 0x080D8AF0) on each panel beside it that isn't
-- its side's (above, below, behind, ahead), each going on that way to the next such panel every 20 ticks (the
-- record's third parameter) and lasting 25; 30 ticks after the shocks it blinks for 30 and goes. It is its side's
-- field object of class 1, with 1 HP: evicted (its HP 0) it leaves a puff and goes; anything its collision meets
-- ends it at once. Its palette is its level's (the record's first parameter, 4 palettes apart).
--
-- In the operation battle's mode (never in a netbattle) it marks its panel as it comes and as it lands (EXE5's effect
-- object 0x83: lib/navi_chips/marker).

local collision = require("../../rules/collision")
local marker = require("../../lib/navi_chips/marker")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local voltz = {}

export type State = {
    -- Its Param word (the record's parameters): the level (byte 0) and the shocks' ticks (byte 2).
    params: number,
}

export type ShockState = {
    -- Its Param word: the sensor's, with the way it goes (byte 1: 0 up, 1 down, 2 back, 3 ahead).
    params: number,
}

-- 0x080D8732: sprite 04-17 with a ground shadow (`sub_8011504`'s 0x01000417), animation 0; a thrown body (row 0x0A)
-- reaching the navis (row 5), hit modifier 3 (0x0A050300).
local SPRITE = asset.sprite("voltz")
local HIT_MOD = 3
-- 0x080D8B22: the shock's sprite 10-11 without a shadow (0x00001011), animation 0; the same collision.
local SHOCK_SPRITE = asset.sprite("thunder-ball")
-- NameID 0xE4: actor record 0xE4 (0x08014C94: version 5, AI index 0x21); its look (0x0801D6B0's row 0x17: the
-- sensor, a ground shadow); EXE5's swallowing and Poltergeist leave it (0x0800D372, 0x080E8CA0).
local IDENTITY = define.identity {
    class = "field_object",
    version = 5,
    ai_index = 0x21,
    scrap = false,
    throwable = false,
    look = { sprite = SPRITE, anim = 0, palette = 0, shadow = true },
}
local HP = 1
-- It drops from 160 pixels, 4 a tick; its collision is on from 16 pixels up.
local DROP_HEIGHT, DROP_SPEED, LOW = 0xA0_0000, 0x4_0000, 16
local DROP_SOUND, SHOCK_SOUND = asset.sound("voltz-drop"), asset.sound("erase-man-2")
-- The effects (EXE5's table, 0x080DFC94): row 0x14 (14-0D, animation 2) when evicted.
local PUFF = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
-- Landed, shocking, blinking out.
local LANDED_TICKS, SHOCK_TICKS, BLINK_TICKS = 10, 30, 30
-- The marker (the operation battle's): 30 ticks, style 9 as it lands.
local MARK_TICKS, LANDED_STYLE = 30, 9
-- The shocks' ways (0x080D8944, and 0x080D8BE0 the same: x by the facing, y): up, down, back, ahead.
local WAYS = { [0] = { 0, -1 }, { 0, 1 }, { -1, 0 }, { 1, 0 } }
-- A shock lasts its ticks and 5 more.
local SHOCK_EXTRA = 5

-- The actions (0x080D87C4).
local DROP, LAND, SHOCK, BLINK = 0, 4, 8, 0xC

local function next_action(me: Object, action: number)
    me.action = action
    me.phase, me.phase_init = 0, 0
end

-- Count the timer down; true when it runs out.
local function count(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- `sub_8011504`: the sprite (its shadow, animation 0 to be loaded) and the collision (false when no slot was free).
local function setup(me: Object, sprite_asset: SpriteAsset, shadow: Shadow): boolean
    local sprite = me.sprite
    sprite:load(sprite_asset)
    sprite:set_animation(0)
    sprite.shadow = shadow
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        return false
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    return true
end

-- Its damage word (+0x2C: the damage and the stamina's halfword).
local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- The panel (x, y) is on the field and not `me`'s side's.
local function other_sides(me: Object, x: number, y: number): boolean
    if not field.valid(x, y) then
        return false
    end
    local p = field.panel(x, y) :: Panel
    return p.alliance ~= me.alliance
end

-- ---- The shock (attack object #0x92) ---------------------------------------------------------------------------

local shock_kind: Kind

-- 0x080D8B82: a shock on (x, y) of `owner`'s side, of `element`, with Param word `params` and the damage word
-- `damage`. (It spawns at the registers' values, which its init replaces: the row, the element and `z`.)
local function spawn_shock(owner: Object, x: number, y: number, element: number, z: number, params: number,
    damage: number): Object?
    local o = battle.spawn(shock_kind, Vec3.new(y, element, z))
    if o then
        (o.state :: ShockState).params = params
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- 0x080D8B98: the next panel its way, if it isn't its side's.
local function next_panel(me: Object): (number?, number?)
    local way = WAYS[bit32.band(bit32.rshift((me.state :: ShockState).params, 8), 0xFF)]
    if not way then
        error("Voltz's shock goes past its ways (0x080D8BE0)")
    end
    local x, y = me.panel_x + way[1] * me.facing, me.panel_y + way[2]
    if not other_sides(me, x, y) then
        return nil, nil
    end
    return x, y
end

-- 0x080D8B40: its hits (a hit ends them); on its ticks the next shock its way; gone after its ticks and 5.
local function shock_tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left == 0 then
        local x, y = next_panel(me)
        if x and y then
            -- (The spawner's r3: the panel lookup's row less 1.)
            spawn_shock(me, x, y, me.element, y - 1, (me.state :: ShockState).params, damage_word(me))
        end
    end
    local life = me.timer2 - 1
    me.timer2 = int.u16(life)
    if life == 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end
    c:present()
end

-- 0x080D8B14: on its panel on the ground, its sprite and collision; its ticks (the third parameter) and 5 more.
local function shock_init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    -- (With no collision slot free the original goes on with none; the engine frees it.)
    if not setup(me, SHOCK_SPRITE, "with_sprite") then
        me:free()
        return
    end
    me.visible = true
    local ticks = bit32.band(bit32.rshift((me.state :: ShockState).params, 16), 0xFF)
    me.timer = ticks
    me.timer2 = ticks + SHOCK_EXTRA
    me:set_lifecycle("update")
    shock_tick(me)
end

shock_kind = define.kind {
    id = "voltz/shock",
    pool = "attack",
    state = { params = "u32" },
    -- 0x080D8AF0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            shock_init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            shock_tick(me)
        else
            me:destroy()
            if not me.active then
                return
            end
        end
        me:update_sprite()
    end,
}

-- ---- The sensor (attack object #0x90) --------------------------------------------------------------------------

-- Gone: no collision, its panel free, no longer a field object.
local function leave(me: Object)
    me.collision:set_region(nil)
    me:unreserve_panel(me.panel_x, me.panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me:set_lifecycle("destroy")
end

-- 0x080D87D4: it drops 4 pixels a tick, its collision on its panel (and the panel reserved) from 16 pixels up;
-- landed, on.
local function drop(me: Object)
    local p = me.pos
    local z = p.z - DROP_SPEED
    if z <= 0 then
        me.pos = Vec3.new(p.x, p.y, 0)
        next_action(me, LAND)
        return
    end
    me.pos = Vec3.new(p.x, p.y, z)
    if bit32.rshift(z, 16) <= LOW then
        me.collision:set_region(regions.single)
        me:reserve_panel(me.panel_x, me.panel_y)
    end
end

-- 0x080D8804: landed (the operation battle's marker), 10 ticks.
local function land(me: Object)
    if me.phase_init == 0 then
        marker.spawn(me, me.panel_x, me.panel_y, MARK_TICKS, LANDED_STYLE, true)
        me.timer = LANDED_TICKS
        me.phase_init = 4
    end
    if count(me) then
        next_action(me, SHOCK)
    end
end

-- 0x080D88E4: a shock on each panel beside it that isn't its side's, in the ways' order, each going its way.
local function shock_around(me: Object)
    local params = (me.state :: State).params
    for i = 0, 3 do
        local way = WAYS[i]
        local x, y = me.panel_x + way[1] * me.facing, me.panel_y + way[2]
        if other_sides(me, x, y) then
            spawn_shock(me, x, y, me.element, params, int.u32(bit32.lshift(i, 8) + params), damage_word(me))
        end
    end
end

-- 0x080D8832: its second animation, the shocks, the sound, its first palette; 30 ticks.
local function shock(me: Object)
    if me.phase_init == 0 then
        me.anim = 1
        shock_around(me)
        battle.play_sound(SHOCK_SOUND)
        me.sprite.palette = 0
        me.timer = SHOCK_TICKS
        me.phase_init = 4
    end
    if count(me) then
        next_action(me, BLINK)
    end
end

-- 0x080D8868: it blinks (hidden while the timer's bit 1 is set) for 30 ticks, and goes.
local function blink(me: Object)
    if me.phase_init == 0 then
        me.timer = BLINK_TICKS
        me.phase_init = 4
    end
    me.visible = not bit32.btest(me.timer, 2)
    if count(me) then
        leave(me)
    end
end

-- 0x080D876E: what its collision met ends it; evicted, a puff; else its action.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        leave(me)
    elseif me.hp == 0 then
        battle.effect(me.pos, PUFF)
        leave(me)
    else
        local a = me.action
        if a == DROP then
            drop(me)
        elseif a == LAND then
            land(me)
        elseif a == SHOCK then
            shock(me)
        elseif a == BLINK then
            blink(me)
        else
            error(string.format("Voltz's sensor action %#x reads past its table (0x080D87C4)", a))
        end
    end
    c:present()
end

-- 0x080D870C: on its panel (off the field it goes at once), 160 pixels up, its sprite and collision (its region
-- off), name, level's palette, 1 HP and its element's spark; the sound and the operation battle's marker.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, DROP_HEIGHT)
    -- (With no collision slot free the original goes on with none; the engine frees it.)
    if not setup(me, SPRITE, "ground") then
        obstacle.unregister(me)
        me:free()
        return
    end
    me.identity = IDENTITY
    me.sprite.palette = bit32.band((me.state :: State).params, 0xFF) * 4
    me.collision:set_region(nil)
    me.visible = true
    me.hp, me.max_hp = HP, HP
    -- 0x08016C42: the spark of its element.
    me.collision:set_hit_effect(sparks.of_element(me.element))
    battle.play_sound(DROP_SOUND)
    marker.spawn(me, me.panel_x, me.panel_y, MARK_TICKS, 1, false)
    me:set_lifecycle("update")
    tick(me)
end

voltz.kind = define.kind {
    id = "voltz/sensor",
    pool = "attack",
    state = { params = "u32" },
    -- 0x080D86E8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            if not me.active then
                return
            end
        end
        me:update_sprite()
    end,
}

-- 0x080D88B0: the sensor on (x, y) of `user`'s side, of `element`, with the record's parameters and the damage word;
-- its side's field object of class 1. (It spawns at the registers' values, which its init replaces: the row, the
-- element and the user's Z.)
local function spawn(user: Object, x: number, y: number, spec: InstantChipSpec, params: number): Object?
    local o = battle.spawn(voltz.kind, Vec3.new(y, spec.element, spec.z))
    if o then
        (o.state :: State).params = params
        o.panel_x, o.panel_y = x, y
        o.element = spec.element
        o:set_damage_word(spec.damage)
        o.alliance, o.flip = user.alliance, user.flip
        obstacle.register(o, o.alliance, 1)
    end
    return o
end

-- The chips' `instant` hook (0x080D88CE), with the record's parameters (its level, 0, the shocks' ticks): the sensor
-- three panels ahead of the user.
function voltz.instant(level: number, shock_ticks: number): (user: Object, spec: InstantChipSpec) -> ()
    local params = bit32.bor(level, bit32.lshift(shock_ticks, 16))
    return function(user: Object, spec: InstantChipSpec)
        spawn(user, spec.panel_x + 3 * user.facing, spec.panel_y, spec, params)
    end
end

return voltz
