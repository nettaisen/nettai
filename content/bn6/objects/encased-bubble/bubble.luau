--!strict
-- The bubble an obstacle encased in a bubble becomes (the original's
-- attack object #0xA3, `sub_80D984C`; docs/engine/field-objects.md §4.5):
-- an aqua obstacle that bursts when a body or another obstacle touches it,
-- and, pushed, rolls toward the enemy 10 pixels a tick until it leaves
-- solid ground. Only the encasing reaches it (`hooks.encased`,
-- rules/roles.luau), and nothing in the game encases: unverified.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local rock = require("../rock/rock")
local rock_debris = require("../rock-debris/rock_debris")
local sparks = require("../../lib/sparks")

local bubble = {}

export type State = {
    -- Its look (the original's first parameter): 0 bubbling up, then 2
    -- (formed); the encasing's starts formed (1).
    look: number,
}

local SPRITE = asset.sprite("immobilized")
local SPEED = 0xA_0000
local FORMED = 2
local SOUND_FORM = asset.sound("place")
local SOUND_BURST = asset.sound("falling-rock")
-- What bursts it: bodies and neutral objects.
local BURSTS_ON = 0x0C80_0000
-- A push hit (hit modifier bits 0x3C).
local PUSHES = 0x3C
-- What it is and reaches: a neutral obstacle that everything reaches,
-- hit modifier 3, the aqua spark.
local IS = define.collision { id = "encased-bubble", side0 = 0x00810082, side1 = 0x00810082, row_offset = 0x70 }
local HITS, HIT_MOD = collision.everything, 3
local SOLID = 0x10

-- `sub_80D99A4`: burst: two debris, bubbles, the sound; gone.
local function burst(me: Object)
    local p = me.pos
    rock_debris.spawn(battle.jitter(0xF, p), 1)
    rock_debris.spawn(battle.jitter(0xF, p), 1)
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.bubbles)
    battle.play_sound(SOUND_BURST)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D9930`: formed (after the bubbling-up animation for look 0), it
-- waits for a push.
local function sit(me: Object)
    local s = me.state :: State
    if s.look == 0 then
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        me.anim = FORMED
        s.look = FORMED
    end
    if bit32.btest(me.collision.hit_mod_final, PUSHES) then
        me:set_action(4)
    end
end

-- `sub_80D995C`: roll; off solid ground, gone in a puff.
local function roll(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    -- sub_800E258: the panel under it, taken only if solid.
    local ox, oy = me.panel_x, me.panel_y
    me:set_panel_from_coordinates()
    if bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
        me:update_collision_panels()
        return
    end
    me.panel_x, me.panel_y = ox, oy
    me.collision:set_region(nil)
    battle.effect(Vec3.new(me.pos.x, me.pos.y, me.pos.z + 0xC_0000), effects.puff)
    me.visible = false
    me:set_lifecycle("destroy")
end

local ACTIONS: { [number]: (Object) -> () } = { [0] = sit, [4] = roll, [8] = burst }

-- `sub_80D98E8`: resolve last tick's hits; a body's or an obstacle's touch
-- bursts it; else its action; off the field, it bursts.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if not bit32.btest(c.hit_flags, BURSTS_ON) then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("the encased bubble's action %#x reads past off_80D9924", me.action))
        end
        action(me)
        if field.valid(me.panel_x, me.panel_y) then
            c:present()
            return
        end
    end
    c:set_region(nil)
    burst(me)
end

-- `sub_80D9870`: on its panel, rolling toward the enemy when pushed. On at
-- once.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.visible = true
    me.anim, me.anim_loaded = s.look * 2, s.look * 2
    sprite:set_animation(s.look * 2)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.vel = Vec3.new(SPEED * (1 - 2 * me.alliance), me.vel.y, me.vel.z)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:set_hit_effect(sparks.of_element(2))
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
    battle.play_sound(SOUND_FORM)
end

bubble.kind = define.kind {
    id = "encased-bubble",
    pool = "attack",
    state = { look = "u8" },
    -- `sub_80D984C`.
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
        if me.active then
            me:update_sprite()
        end
    end,
}

-- `sub_80D99EC`: a formed bubble where `obstacle` was, aqua, with its side
-- and damage word, running while dimmed. Its position is the spawner's
-- registers (panel row, side), which its init overwrites but for Z: there
-- the original leaves what `sub_80E544C` last read, side 1's wind object
-- (0 when side 1 has none, or the encased obstacle was it; otherwise an
-- address, which the port can't give: 0 too, unverified).
function bubble.spawn(obstacle: Object): Object?
    local o = battle.spawn(bubble.kind, Vec3.new(obstacle.panel_y, obstacle.alliance, 0))
    if not o then
        return nil
    end
    (o.state :: State).look = 1
    o.panel_x, o.panel_y = obstacle.panel_x, obstacle.panel_y
    o.element = 2
    o.damage, o.stamina = obstacle.damage, obstacle.stamina
    o.alliance = obstacle.alliance
    o.run_while_dimmed = true
    return o
end

-- The role `hooks.encased` (`sub_801813A`'s end): an obstacle encased in
-- ice becomes an ice block (the rock's variant 3, in the same registry
-- class, there at once), one in a bubble the bubble; at its panel, with its
-- side and damage word. An unregistered obstacle's ice block would take
-- class 0xFF, which `setFieldBattleObject_800F614` turns into a write past
-- the registry (BattleState+0x898 and on): a content error here.
function bubble.replace(obstacle: Object, ice: boolean, class: number?)
    if ice then
        if class == nil then
            error("an unregistered obstacle's ice block registers in class 0xFF, past the field-object registry")
        end
        local damage = bit32.bor(obstacle.damage, bit32.lshift(obstacle.stamina, 16))
        rock.spawn(obstacle.panel_x, obstacle.panel_y, obstacle.alliance, { 3, class, 1, 0 }, damage)
    else
        bubble.spawn(obstacle)
    end
end

return bubble
