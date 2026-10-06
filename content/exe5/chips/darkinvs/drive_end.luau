--!strict
-- The end of DarkInvs's drive: EXE5's action 0x49 (0x080EF188), the roles' `stun_strike` (the request the drive's
-- timer raises at its end, rules/status's `no_charge_drive`). The navi, white, drops the no-charge state and stands
-- (animation 1) for 23 ticks; on its 14th its dark image leaves it (actor object #10, 0x080BA71C: drawn as the navi
-- in the dark palette, sliding 40 pixels ahead, then fading); then a sound, the navi touchable again, the request
-- dropped, the navi flashing for 120 ticks, and back to idle.

-- MegaMan, by key (his navi's module reaches this one through his souls' Chaos Unison weapons).
local MEGAMAN = "megaman"

local drive_end = {}

export type State = {
    -- AIAttackVars+0x10: ticks left.
    timer: number,
}

local SOUND = asset.sound("fade")
-- How long it stands (to 0), when the image leaves (ticks left), and the flash after.
local STAND, IMAGE_AT, FLASH = 0x17, 9, 0x78
-- MegaMan's dark palette (0x0800DE82), and how far ahead the image slides a tick (pixels).
local DARK, SLIDE = 1, 10

-- ---- The dark image (actor object #10) ------------------------------------------------------------------------

-- 0x0800DE82: the dark palette of the navi's (MegaMan's 1; any other navi's dark chip fizzles before the drive).
local function dark_palette(me: Object): number
    if battle.navi(me.alliance).navi.id ~= MEGAMAN then
        error("DarkInvs's dark image: only MegaMan's DarkInvs gets this far (0x0800DE82's link navi palette)")
    end
    return DARK
end

-- Count the timer down from `ticks` (set on the action's first tick); what is left.
local function count(me: Object, ticks: number): number
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left
end

-- Action 0 (0x080BA7E0): 10 pixels ahead a tick while 4 ticks are left (to 0); then it stays.
local function slide(me: Object)
    if count(me, 4) >= 0 then
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
        return
    end
    me:set_action(4)
end

-- Action 4 (0x080BA80C): 10 ticks.
local function stay(me: Object)
    if count(me, 0x0A) <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080BA82E): it fades (animation 4); 4 ticks; its parts off, and it goes.
local function fade(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if count(me, 4) <= 0 then
        me:remove_parts_of(me.related1 :: Object)
        me:set_lifecycle("destroy")
    end
end

local IMAGE_ACTIONS = { [0] = slide, [4] = stay, [8] = fade }

-- 0x080BA7C0.
local function image_tick(me: Object)
    local action = IMAGE_ACTIONS[me.action]
    if not action then
        error(string.format("DarkInvs's dark image: action %#x reads past its table (0x080BA7D4)", me.action))
    end
    action(me)
end

-- 0x080BA740: drawn as a player navi (0x0800D1C0's actor type 2: its sprite with a ground shadow, from animation 0,
-- the dark palette, its parts stepping while paused), facing its way, its X velocity 10 pixels ahead; else gone.
local function image_init(me: Object)
    local navi = me.related1 :: Object
    if navi.actor_type ~= "player" then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load_like(navi)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite.palette = dark_palette(me)
    me.vel = Vec3.new(me.facing * SLIDE * 0x10000, me.vel.y, me.vel.z)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_parts_of(navi, false, true)
    me.visible = true
    me:set_lifecycle("update")
    image_tick(me)
end

drive_end.image = new.kind {
    id = "darkinvs/image",
    pool = "actor",
    -- 0x080BA71C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            image_init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            image_tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080BA85C: the image of `navi` where it stands (its position; its panel, which the code reads into a register
-- rather than stores, left as spawned), of its side and facing, running while dimmed.
local function spawn_image(navi: Object): Object?
    local o = battle.spawn(drive_end.image)
    if o then
        o.related1 = navi
        o.alliance, o.flip = navi.alliance, navi.flip
        o.pos = navi.pos
        o.run_while_dimmed = true
    end
    return o
end

-- ---- The action ---------------------------------------------------------------------------------------------

-- 0x080EF188
local function update(me: Object, s: State)
    -- sprite_forceWhitePalette.
    me.sprite.white = true
    if me.step_init == 0 then
        me:set_navi_state("no_charge", false)
        me:set_animation(1)
        s.timer = STAND
        me.step_init = 4
    end
    if s.timer == IMAGE_AT then
        spawn_image(me)
    end
    local left = s.timer - 1
    s.timer = int.u16(left)
    if left > 0 then
        return
    end
    battle.play_sound(SOUND)
    me:set_status("untouchable", false)
    me:set_request("stun_strike", false)
    -- 0x08017432: flashing for 120 ticks.
    me:set_status_timer("flash", FLASH)
    me:set_status("flashing", true)
    me:exit_attack()
end

drive_end.action = new.action { id = "darkinvs/drive-end", state = { timer = "u16" }, update = update }

return drive_end
