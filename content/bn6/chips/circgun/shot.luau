--!strict
-- CircGun's shot (the original's attack object #0x8A, `sub_80D67EC`;
-- docs/engine/dimming-chip-effects.md §8): the gun's mark on a panel that,
-- 29 ticks later, hits it if it is solid, with a flash, and tells the gun
-- it is done. Its sprite steps while dimmed.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local shot = {}

-- What the gun keeps of its shots (its state): one flag each.
type GunState = { shots_out: { number } }

export type State = {
    -- The gun waiting on it, and its flag there (ExtraVars+4 + i).
    gun: Object?,
    slot: number,
    look: number,
    ticks: number,
}

local SPRITE = asset.sprite("sight")
local ANIM = 1
local SOUND_MARK = asset.sound("mark")
local SOUND_HIT = asset.sound("col-army-2")
-- Its hit (`dword_80D690C`, `byte_80D68D8`, by its look): look 0 a thrown
-- attack, look 1 another attacker's with bug 0x54 (argument 0x32); hitting
-- navis, hit modifier 3, element 0's spark.
local HIT_IS: { [number]: CollisionType } = {
    [0] = collision.thrown,
    [1] = define.collision { id = "circgun/other-hit", side0 = 0x80000488, side1 = 0x40000488, row_offset = 0x158 },
}
local HIT_BUG: { [number]: { number } } = { [0] = { 0, 0 }, [1] = { 0x54, 0x32 } }

-- `sub_80D67D2`: on its panel, 8 pixels down and up.
local function place(me: Object)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y + 0x8_0000, 0x8_0000)
end

-- Done: the gun's flag cleared (as the original, wherever the gun was);
-- freed next update.
local function finish(me: Object)
    local s = me.state :: State
    local gun = s.gun
    if gun and gun.active then
        (gun.state :: GunState).shots_out[s.slot + 1] = 0
    end
    me:set_lifecycle("destroy")
end

-- `sub_80D6874`: 29 ticks.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.anim = ANIM
        me:set_action(4)
    end
end

-- `sub_80D688C`: hit the panel if it is solid.
local function hit(me: Object)
    if field.solid(me.panel_x, me.panel_y) then
        local look = (me.state :: State).look
        local is, bug = HIT_IS[look], HIT_BUG[look]
        if not is or not bug then
            error(string.format("CircGun's shot of look %d reads past byte_80D68D8", look))
        end
        local h = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.of_element(0),
            target = collision.hits_navis,
            self_type = is,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
            bug = bug[1],
            bug_arg = bug[2],
        })
        if h then
            h.run_while_dimmed = true
        end
        battle.effect(me.pos, effects.flash)
        battle.play_sound(SOUND_HIT)
    end
    finish(me)
end

-- `sub_80D685A`.
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        wait(me)
    elseif a == 4 then
        hit(me)
    else
        error(string.format("CircGun's shot: action %#x reads past off_80D686C", a))
    end
end

-- `sub_80D6810`: the mark on its panel, in its look's palette.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = ANIM, 0xFF
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    place(me)
    me.timer = s.ticks
    sprite.palette = s.look
    battle.play_sound(SOUND_MARK)
    me:set_lifecycle("update")
    tick(me)
end

shot.kind = define.kind {
    id = "circgun/shot",
    pool = "attack",
    state = { gun = "object", slot = "u8", look = "u8", ticks = "u8" },
    -- `sub_80D67EC`.
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

-- `sub_80D68E0`: a shot on (x, y) with `element` and the damage word, of
-- `gun`'s side (not its facing), running while dimmed, waiting `ticks`;
-- the gun's flag `slot` is set until it is done.
function shot.spawn(gun: Object, x: number, y: number, element: number, damage: number, ticks: number, look: number, slot: number): Object?
    local o = battle.spawn(shot.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance = gun.alliance
    local s = o.state :: State
    s.gun, s.slot, s.look, s.ticks = gun, slot, look, ticks
    ;(gun.state :: GunState).shots_out[slot + 1] = 1
    o.run_while_dimmed = true
    return o
end

return shot
