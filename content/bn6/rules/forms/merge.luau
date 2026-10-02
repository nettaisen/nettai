--!strict
-- A Cross navi's image merging with MegaMan (actor object #0x1B,
-- `sub_80BC650`), spawned by a Cross change: it appears above and in front
-- of MegaMan, then swings from one side of him to the other, each swing
-- narrower, and when it reaches him it bursts into an effect and is gone.
-- Purely visual, but it holds an actor slot and a place in the update
-- order, and runs while the battle is paused. See
-- docs/engine/objects-and-player.md §12.10.

local effects = require("../../lib/effects")

local merge = {}

export type State = {
    -- Which navi it shows (Param1).
    navi: NaviDef?,
    -- +0x62: swings left; each is `swing_step` narrower than the last.
    swings_left: number,
    -- ExtraVars+4: how much narrower each swing gets.
    swing_step: number,
    -- +0x68: the side of the next swing (+1 ahead of MegaMan, -1 behind).
    swing_side: number,
    -- ExtraVars+0xC: how far above the panel it floats (added to both Y
    -- and Z, so it is drawn at the panel's height, nearer the viewer).
    lift: number,
    -- ExtraVars+0x10: the navi's extra height (its `merge_height`).
    extra_height: number,
    -- ExtraVars+0x14: the swing sound has played.
    sounded: boolean,
}

local CROSS_MERGE = asset.sound("bn6:cross-merge")

-- `object_getFlipDirection`: +1 when facing right, -1 when facing left.
local function facing(o: Object): number
    return if bit32.bxor(o.alliance, o.flip) == 0 then 1 else -1
end

-- `sub_80BC670`: the navi's sprite, ahead of MegaMan by a full swing.
local function init(me: Object, s: State)
    local navi = s.navi :: NaviDef
    local identity = navi.identity
    me.identity = identity
    -- sub_800FC9E(navi, no form): the navi's own sprite.
    me.sprite:load(navi.sprite)
    me.sprite:set_animation(0)
    -- sprite_removeShadow, palette 4, the object's facing, and white
    -- (sprite_forceWhitePalette at the end of the init).
    me.sprite.shadow = "hidden"
    me.sprite.palette = 4
    me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.sprite.white = true
    me.no_sprite_update = false
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    local swings = s.swings_left
    -- The game's svc Div, with a zero divisor never passed.
    local step = int.tdiv(0x28_0000, swings)
    local lift = (4 - me.panel_y) * 0x18_0000
    local extra = (navi.merge_height or 0) * 0x10000
    s.swing_side = -1
    s.swing_step = step
    s.lift = lift
    s.extra_height = extra
    s.sounded = false
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + int.mul32(int.mul32(step, swings), facing(me)), p.y + lift, lift + extra)
    -- sub_8010DD0: the navi's own init hook (some wear an overlay).
    if identity then
        me:add_parts(identity, 0)
    end
    -- SpoutMan's image keeps its idle overlay at its own height (the
    -- hook's result, ExtraVars[0] = 1; the original tests his navi number:
    -- the one navi that wears an idle overlay).
    if identity and identity.parts and identity.parts.idle and me.related2 then
        (me.related2 :: Object):pin_overlay()
    end
    me:set_lifecycle("update")
end

-- It reached MegaMan: its NameID's death hook (`sub_8011020`: its overlay,
-- if its init hook put one on, comes off), an effect on the panel, and it
-- is freed at once.
local function burst(me: Object)
    local identity = me.identity
    if identity then
        me:remove_parts(identity)
    end
    local x, y = field.center(me.panel_x, me.panel_y)
    local e = battle.effect(Vec3.new(x, y, 0x10_0000), effects.deletion, 0, 0, 0)
    if e then
        e.run_while_paused = true
    end
    me:free()
end

-- `sub_80BC78C`: hold, then swing every other tick until it reaches
-- MegaMan.
local function tick(me: Object, s: State)
    if me.action == 0 then
        -- Held in place, white, until the timer runs out.
        me.sprite.white = true
        me.timer -= 1
        if me.timer ~= 0 then
            return me:update_sprite()
        end
        -- sprite_clearFinalPalette
        me.sprite.white = false
        me.timer = 10
        me.action = 1
    end
    me.timer -= 1
    if me.timer ~= 0 then
        return me:update_sprite()
    end
    if not s.sounded then
        s.sounded = true
        battle.play_sound(CROSS_MERGE)
    end
    s.swings_left -= 1
    if int.i16(s.swings_left) <= 0 then
        return burst(me)
    end
    me.timer = 2
    me:set_coordinates_from_panel()
    local dx = int.mul32(int.mul32(int.mul32(s.swing_side, s.swing_step), s.swings_left), facing(me))
    local p = me.pos
    me.pos = Vec3.new(p.x + dx, p.y + s.lift, s.lift + s.extra_height)
    s.swing_side = -s.swing_side
    me:update_sprite()
end

merge.kind = define.kind {
    id = "bn6:forms/cross-merge",
    pool = "actor",
    state = {
        navi = "navi",
        swings_left = "u16",
        swing_step = "i32",
        swing_side = "i32",
        lift = "i32",
        extra_height = "i32",
        sounded = "bool",
    },
    update = function(me: Object)
        local s = me.state :: State
        local l = me.lifecycle
        if l == "init" then
            init(me, s)
        elseif l == "update" then
            tick(me, s)
        else
            me:free()
        end
    end,
}

-- `sub_80BC844`: `navi`'s image over MegaMan (`owner`), `swings` swings
-- from his panel.
function merge.spawn(owner: Object, navi: NaviDef, swings: number): Object?
    local o = battle.spawn(merge.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
    o.alliance = owner.alliance
    o.flip = owner.flip
    o.related1 = owner
    o.timer = 6
    o.run_while_paused = true
    o.run_while_dimmed = true
    local s = o.state :: State
    s.navi = navi
    s.swings_left = swings
    return o
end

return merge
