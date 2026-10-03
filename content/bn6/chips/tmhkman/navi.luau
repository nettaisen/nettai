--!strict
-- TomahawkMan, the navi of the TmhkMan chips (the original's actor object
-- #0x0A, `sub_80B97C0`), brought by their navi chip controller. He
-- appears, raises his tomahawk and brings it down on the panel ahead of
-- him: a hit region two columns wide and three rows tall from there, a
-- splash of earth and a camera shake. Then he leaves, and the controller
-- moves on. See docs/engine/chips.md §3.6.12.

local navi_chips = require("@common/navi-chips/navi")
local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local tmhkman = {}

local SPRITE = asset.sprite("bn6:tomahawkman")
local SOUND_APPEAR = asset.sound("bn6:appear")
local SOUND_TOMAHAWK_MAN = asset.sound("bn6:tomahawk-man")

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B999B

-- When his tomahawk lands (ticks left of his swing).
local STRIKE = 0x37
-- The earth it splashes (effect 0x33).
local EARTH = define.effect { sprite = asset.sprite("bn6:big-slash"), anim = 1, palette = 0 }

-- `sub_80B97E4`: on his panel, on the ground (Z's whole part cleared).
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground")
    me:set_lifecycle("update")
end

-- `sub_80B9848`: he appears; 3 ticks (Timer and Timer2 a word); on to
-- standing, or straight to leaving if his panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer, me.timer2 = 3, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then 4 else 0x10)
end

-- `sub_80B989C`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0
        me.timer, me.timer2 = 0x1E, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(8)
    end
end

-- `sub_80B98C4`: 30 ticks raising the tomahawk.
local function raise(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0xF
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0xC)
    end
end

-- The strike: a hit region on the panel ahead running while dimmed
-- (`sub_80C53A6`: no hit spark), the earth splashing there
-- (16 pixels up), sound 0x10A, and the camera shakes (magnitude 1, 30
-- ticks).
local function strike(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.wide_long,
        hit_effect = nil,
        target = collision.nothing,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 1,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), EARTH, bit32.bxor(me.alliance, me.flip))
    battle.play_sound(SOUND_TOMAHAWK_MAN)
    battle.shake_camera(1, 0x1E)
end

-- `sub_80B98EC`: the swing, 61 ticks (to -1); the tomahawk lands at 55
-- left.
local function swing(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 0x10
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == STRIKE then
        strike(me)
        return
    end
    if left < 0 then
        me:set_action(0x10)
    end
end

-- `sub_80B9970`: 3 ticks, then he goes and the controller moves on.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 4
        me.timer, me.timer2 = 3, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = raise, [0xC] = swing, [0x10] = leave }

tmhkman.kind = define.kind {
    id = "bn6:tmhkman/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80B97C0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("TomahawkMan action %#x reads past his table (off_80B9834)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80B999A`: TomahawkMan for `user`, reporting to `controller` (the
-- TmhkMan chips' navi hook, `off_802CD5C[8]`). His position is the
-- spawner's registers until his init places him.
function tmhkman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(tmhkman.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return tmhkman
