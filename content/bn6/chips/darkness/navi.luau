--!strict
-- The Darkness Program Advance's navis, Dark MegaMan and Bass: one kind
-- (the original's actor object #0x25, `sub_80BF6EC`), told apart by `bass`
-- (its Param1). The navi chip controller brings Dark MegaMan in his
-- user's stead: he fades in, raises his aura and three waves of dark
-- flames ahead of him, and vanishes; then Bass swoops in, charges and
-- slashes, and fades out. See docs/engine/chips.md §3.6.36.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local element_pillar = require("../../objects/element-pillar/element_pillar")
local navi_chips = require("../../lib/navi-chips/navi")
local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")

local darkness = {}

export type State = {
    -- Bass (Param1 1), or Dark MegaMan.
    bass: boolean,
    -- What he holds (ExtraVars[0]): Dark MegaMan's aura, Bass's sword.
    held: Object?,
    -- Dark MegaMan: the controller waiting for him; Bass: Dark MegaMan,
    -- who waits for him (ExtraVars[1], the byte each clears when he goes:
    -- the controller's flag, Dark MegaMan's FuturePanelX).
    controller: Object?,
    leader: Object?,
}

-- How Bass looks (`byte_80BF7A8`): his sprite and his actor record's
-- parts (NameID 0x16D's: his cape). Dark MegaMan wears MegaMan's identity,
-- in his base form (NameID 0x1A0: no parts), whose attach points his aura
-- takes.
local BASS_SPRITE = asset.sprite("bass-anly")
local BASS_ACTOR: ActorType, BASS_AI = "navi", 0x13
local BASE_FORM = 0

local SOUND_DARK, SOUND_APPEAR, SOUND_ROAR = asset.sound("immobilizer"), asset.sound("appear"), asset.sound("roar")
local SOUND_SWOOP, SOUND_SLASH = asset.sound("swoop"), asset.sound("big-sword-swing")
local SOUND_CHARGE, SOUND_CHARGED = asset.sound("buster-charge"), asset.sound("buster-charged")

-- Dark MegaMan's palette, and his aura (attachment 0x32).
local DARK_PALETTE = 0x19
local AURA = attachment.look { sprite = asset.sprite("dark-aura"), palette = 0, attach_point = 24 }

-- The dark flames (`sub_80BFD02`): element pillars (objects/element-pillar;
-- the original's fifth kind: flames that stand back by their wave and last
-- through his own action 4), fire, 6 pixels up, by wave (`off_80BFD68`: the
-- panels from his, dx toward the front); they last 0x5A ticks and 8 per
-- wave still to come, then 8 once he is done.
local FLAME_ELEMENT, FLAME_Z, FLAME_TICKS = 1, 0x6_0000, 0x5A
local FLAME_PANELS = {
    [0] = { { 1, 0 } },
    [1] = { { 2, -1 }, { 2, 0 }, { 2, 1 } },
    [2] = { { 3, -1 }, { 3, 0 }, { 3, 1 } },
}

-- Bass's charge: the glow while it builds (effect 0x4E) and the flash
-- when it is full (0x4F), at his sprite's attach point 0 (`sub_8018810` by
-- his NameID: his sprite's table row, whose point 0 is (0, 16)).
local CHARGING = define.effect { sprite = asset.sprite("charge-glow"), anim = 0, palette = 0 }
local CHARGED = define.effect { sprite = asset.sprite("charge-glow"), anim = 2, palette = 0 }
local GLOW_X, GLOW_Z = 0, 16

-- Bass's afterimages (`sub_80BFD90`): their colour shader and lifetime.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x8108, 0xF

-- The controller calls Dark MegaMan's spawner (`sub_80BFCD0`) through r3,
-- which his position takes (his init sets it all).
local SPAWNER_ADDRESS = 0x080BFCD1

-- Dark MegaMan's actions (`off_80BF7E4`); Bass's (`off_80BF7F4`).
local APPEAR, FLAMES, VANISH, BRING_BASS = 0x0, 0x4, 0x8, 0xC
local SWOOP, STRIKE, FADE = 0x0, 0x4, 0x8

-- How long his phase has left, one tick down (a halfword timer).
local function tick_down(me: Object): number
    local left = me.timer - 1
    me.timer = left
    return left
end

-- Count down from `ticks` entered on the phase's first tick (counting that
-- tick too); what is left.
local function count(me: Object, ticks: number): number
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    return tick_down(me)
end

-- The game's flip value for his sprite.
local function flip_of(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- `sub_80B8E70`: the held attachment's animation, from its start.
local function held_anim(me: Object, anim: number)
    local held = (me.state :: State).held
    if held then
        held.anim = anim
        held.anim_loaded = 0xFF
    end
end

-- `sub_80B8E58`: what he holds goes (its lifecycle word to destroy); the
-- slot keeps it.
local function let_go(me: Object)
    local held = (me.state :: State).held
    if held then
        held:set_lifecycle("destroy")
    end
end

-- ---- Dark MegaMan ---------------------------------------------------------

-- `sub_80BF81C`: in his dark palette he fades in, a step every other tick
-- over 32 ticks.
local function fade_in(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_DARK)
        me.timer, me.timer2 = 0, 0
        me.sprite.palette = DARK_PALETTE
        me.phase_init = 4
    else
        local t = me.timer + 1
        me.timer = t
        if t > 1 then
            me.timer = 0
            local step = me.timer2 + 1
            me.timer2 = step
            if step > 0xF then
                me.sprite.alpha = nil
                me.phase, me.phase_init = 4, 0
                return
            end
        end
    end
    me.sprite.alpha = me.timer2
end

-- `sub_80BF866`: 30 ticks.
local function appeared(me: Object)
    if count(me, 0x1E) <= 0 then
        me:set_action(FLAMES)
    end
end

-- `sub_80BF8A8`: his aura; 30 ticks.
local function raise_aura(me: Object)
    if me.phase_init == 0 then
        me.anim = 0xA
        attachment.attach(me, AURA, "held", { anim = 0, while_dimmed = true })
        battle.play_sound(SOUND_APPEAR)
    end
    if count(me, 0x1E) <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BFD02(wave)`: the wave's dark flames (they run while dimmed).
local function dark_flames(me: Object, wave: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local flames: ElementPillar = { ticks = (2 - wave) * 8 + FLAME_TICKS, after = 8, back = wave, owner_state = FLAMES }
    for _, d in FLAME_PANELS[wave] do
        local x, y = me.panel_x + d[1] * me.facing, me.panel_y + d[2]
        local pillar = element_pillar.spawn(me, x, y, FLAME_ELEMENT, flames, damage, FLAME_Z)
        if pillar then
            pillar.run_while_dimmed = true
        end
    end
end

-- `sub_80BF8DA`: the aura flares (a long shake, a roar every 16 ticks);
-- the flames' waves at 0, 8 and 16; past 106, on.
local function flare(me: Object)
    if me.phase_init == 0 then
        me.timer, me.timer2 = 0, 0
        held_anim(me, 2)
        battle.shake_camera(2, 0x6A)
        me.phase_init = 4
    end
    if me.timer2 == 0 then
        battle.play_sound(SOUND_ROAR)
    end
    local t2 = me.timer2 + 1
    me.timer2 = if t2 >= 0x10 then 0 else t2
    local t = me.timer
    if t == 0 or t == 8 or t == 0x10 then
        dark_flames(me, t // 8)
    end
    t += 1
    me.timer = t
    if t > 0x6A then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BF94E`: the aura dies down; 30 ticks.
local function aura_down(me: Object)
    if me.phase_init == 0 then
        held_anim(me, 3)
    end
    if count(me, 0x1E) <= 0 then
        me:set_action(VANISH)
    end
end

-- `sub_80BF888`.
local FLAMES_PHASES: { [number]: (me: Object) -> () } = { [0] = raise_aura, [4] = flare, [8] = aura_down }

-- `sub_80BF97C`: the aura goes; 3 ticks, then out of sight.
local function vanish(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        let_go(me)
    end
    if count(me, 3) <= 0 then
        me.visible = false
        me:set_action(BRING_BASS)
    end
end

-- `sub_80BF9CC`: Bass, for his user, on his panel, with no element (his
-- FuturePanelX the flag Bass clears when he goes); once Bass is gone, on.
local function bring_bass(me: Object)
    if me.phase_init == 0 then
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local spec = { panel_x = me.panel_x, panel_y = me.panel_y, element = 0, damage = damage }
        local bass = navi_chips.spawn(darkness.kind, me.related1 :: Object, spec, 0)
        if bass then
            bass.run_while_dimmed = true
            local s = bass.state :: State
            s.bass = true
            s.leader = me
            me.future_panel_x = 1
        end
        me.phase_init = 4
    end
    if me.future_panel_x == 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BF9FE`: 30 ticks, then he goes.
local function after_bass(me: Object)
    if count(me, 0x1E) <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80BF9B0`.
local BRING_PHASES: { [number]: (me: Object) -> () } = { [0] = bring_bass, [4] = after_bass }

-- `sub_80BF800`.
local APPEAR_PHASES: { [number]: (me: Object) -> () } = { [0] = fade_in, [4] = appeared }

-- ---- Bass -------------------------------------------------------------------

-- `sub_80BFD90(anim)`: two afterimages a pixel up the field and down: his
-- sprite at animation 0x14 + `anim` (its shadow at its height), then at
-- `anim` (a ground shadow).
local function afterimages(me: Object, anim: number)
    local p = me.pos
    local at = Vec3.new(p.x, p.y - 0x10000, p.z - 0x10000)
    local flip = flip_of(me)
    battle.afterimage(me, at, {
        sprite = BASS_SPRITE,
        anim = 0x14 + anim,
        flip = flip,
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
    })
    battle.afterimage(me, at, {
        sprite = BASS_SPRITE,
        anim = anim,
        flip = flip,
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
        shadow = "ground",
    })
end

-- `sub_80BFA38`: he swoops in from 100 pixels behind and above, 10 a tick
-- (afterimages at 5 and 7); at 10 onto his panel.
local function swoop(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_SWOOP)
        me.timer = 0
        local p = me.pos
        me.pos = Vec3.new(p.x - me.facing * 0x64_0000, p.y, 0x64_0000)
        me.vel = Vec3.new(me.facing * 0xA_0000, me.vel.y, -0xA_0000)
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y, p.z + v.z)
    if me.timer == 5 or me.timer == 7 then
        afterimages(me, 0)
    end
    local t = me.timer + 1
    me.timer = t
    if t >= 0xA then
        me:set_coordinates_from_panel()
        me.sprite.alpha = nil
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BFAAE`: a tick.
local function landed(me: Object)
    if count(me, 0) <= 0 then
        me:set_action(STRIKE)
    end
end

-- `sub_80BFA1C`.
local SWOOP_PHASES: { [number]: (me: Object) -> () } = { [0] = swoop, [4] = landed }

-- An effect at his attach point 0.
local function at_hand(me: Object, effect: Effect): Object?
    local p = me.pos
    return battle.effect(Vec3.new(p.x + GLOW_X * me.facing * 0x10000, p.y, p.z + GLOW_Z * 0x10000), effect)
end

-- `sub_80BFAF8`: his charge: the glow, at 20 left the flash; 40 ticks.
local function charge(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        local glow = at_hand(me, CHARGING)
        -- (A failed spawn: the game writes the timer into the BIOS.)
        if glow then
            glow.timer = 0x1E
        end
        battle.play_sound(SOUND_CHARGE)
        me.timer = 0x28
        me.phase_init = 4
    end
    if me.timer == 0x14 then
        local flash = at_hand(me, CHARGED)
        if flash then
            flash.timer = 0xA
        end
        battle.play_sound(SOUND_CHARGED)
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BFB74`: the slash (his sword); at 10 the hit on the panel ahead
-- and the one beyond (no element), the big slash in the dark palette 16
-- pixels up, a shake and a flash; past 30, on.
local function slash(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
        me.timer = 0
        attachment.attach(me, parts.blades.sword, "held", { anim = 0xE, while_dimmed = true })
        battle.play_sound(SOUND_SLASH)
        me.phase_init = 4
    end
    if me.timer == 0xA then
        local x, y = me.panel_x + me.facing, me.panel_y
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.element,
            z = 0,
            region = regions.wide_long,
            hit_effect = 0xFF,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if box then
            box.run_while_dimmed = true
        end
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), parts.effects.big, flip_of(me), 4)
        battle.shake_camera(2, 0x28)
        battle.palette_flash(0, 0x1E, true, false)
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0x1E then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BFBFA`: 30 ticks.
local function after_slash(me: Object)
    if count(me, 0x1E) <= 0 then
        me:set_action(FADE)
    end
end

-- `sub_80BFAD8`.
local STRIKE_PHASES: { [number]: (me: Object) -> () } = { [0] = charge, [4] = slash, [8] = after_slash }

-- `sub_80BFC44`: his sword goes; he fades out, a step every third tick.
local function fade_out(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        let_go(me)
        me.timer, me.timer2 = 0, 0xF
        me.phase_init = 4
    else
        local t = me.timer + 1
        me.timer = t
        if t > 2 then
            me.timer = 0
            local step = me.timer2 - 1
            me.timer2 = step
            if step <= 0 then
                me.visible = false
                me.sprite.alpha = nil
                me.phase, me.phase_init = 4, 0
                return
            end
        end
    end
    me.sprite.alpha = me.timer2
end

-- `sub_80BFC94`: 30 ticks, then he goes.
local function faded(me: Object)
    if count(me, 0x1E) <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80BFC28`.
local FADE_PHASES: { [number]: (me: Object) -> () } = { [0] = fade_out, [4] = faded }

-- ---- Both ---------------------------------------------------------------

-- Run an action's phase from its table.
local function run_phase(me: Object, phases: { [number]: (Object) -> () }, what: string)
    local phase = phases[me.phase]
    if not phase then
        error(string.format("Darkness %s phase %#x reads past its table", what, me.phase))
    end
    phase(me)
end

-- `sub_80BF7BC`: Dark MegaMan's actions or Bass's.
local function act(me: Object)
    local action = me.action
    if (me.state :: State).bass then
        if action == SWOOP then
            run_phase(me, SWOOP_PHASES, "Bass's swoop")
        elseif action == STRIKE then
            run_phase(me, STRIKE_PHASES, "Bass's strike")
        elseif action == FADE then
            run_phase(me, FADE_PHASES, "Bass's fade")
        else
            error(string.format("Darkness Bass's action %#x reads past his table (off_80BF7F4)", action))
        end
        return
    end
    if action == APPEAR then
        run_phase(me, APPEAR_PHASES, "Dark MegaMan's appearing")
    elseif action == FLAMES then
        run_phase(me, FLAMES_PHASES, "Dark MegaMan's flames")
    elseif action == VANISH then
        vanish(me)
    elseif action == BRING_BASS then
        run_phase(me, BRING_PHASES, "Dark MegaMan's waiting")
    else
        error(string.format("Darkness Dark MegaMan's action %#x reads past his table (off_80BF7E4)", action))
    end
end

-- `sub_80BF710`: his sprite (animation 0, loaded on the tick's sprite
-- update), on his user's place (kept as his velocity, which Bass's swoop
-- overwrites), on the ground, his parts. Then his first update at once.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    if s.bass then
        sprite:load(BASS_SPRITE)
        sprite.shadow = "ground"
        me.anim = 0
        me.anim_loaded = 0xFF
    else
        me:wear_megaman_image(BASE_FORM)
    end
    local p = (me.related1 :: Object).pos
    me.pos = p
    me.vel = p
    me:set_panel_from_coordinates()
    me.pos = Vec3.new(p.x, p.y, 0)
    me.visible = true
    sprite:set_flip(flip_of(me))
    if s.bass then
        me:add_navi_parts(BASS_ACTOR, BASS_AI, 1)
    else
        me:navi_image_parts(true)
    end
    me:set_lifecycle("update")
    act(me)
end

-- `sub_80BFCB2`: his parts off; whoever waits for him moves on (Dark
-- MegaMan's controller; Bass's Dark MegaMan); gone.
local function destroy(me: Object)
    local s = me.state :: State
    if s.bass then
        me:remove_navi_parts(BASS_ACTOR, BASS_AI)
    else
        me:navi_image_parts(false)
    end
    if s.controller then
        navi_chip.navi_left(s.controller)
    end
    local leader = s.leader
    if leader then
        leader.future_panel_x = 0
    end
    me:free()
end

darkness.kind = define.kind {
    id = "darkness/navi",
    pool = "actor",
    state = { bass = "bool", held = "object", controller = "object", leader = "object" },
    -- `sub_80BF6EC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            destroy(me)
        end
        me:update_sprite()
    end,
}

-- The Darkness chip's `navi` hook (`off_802CD5C[24]`): Dark MegaMan for the
-- user, reporting to `controller`; he brings Bass.
function darkness.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(darkness.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    o.run_while_dimmed = true
    (o.state :: State).controller = controller
    return o
end

return darkness
