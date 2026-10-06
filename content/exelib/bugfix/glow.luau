--!strict
-- The glow over a navi, as EXE5 and EXE6 share it (EXE6's actor object #0x5D, `sub_80C4828`; EXE5's #0x5D, 0x080C42E8,
-- the same code but for its start; EXE6's docs/engine/dimming-chips.md §5.2): a copy of the side's navi, which
-- vanishes under it, flashing white. Its variant (the original's first parameter, `off_80C4930`'s rows) is its
-- spawner's: BugFix's controller's
--
--   bug_fix (`sub_80C493C`, the first): white flashes further and further apart for 120 ticks, then 30 ticks, and the
--     navi is back;
--
-- or EXE5's Liberation chips'
--
--   log_in (`sub_80C4A52`, the third): white for 30 ticks with the log-in sound, then as bug_fix's end.
--
-- (The second, `sub_80C49E4`, which leaves the navi submerged for 480 ticks after 31 white, nothing spawns.)
--
-- When it is done it clears its spawner's `busy` flag. Each game makes its own kind (`glow.make`) with its look: its
-- sounds and how its start (EXE6's `sub_80C4848`, EXE5's 0x080C4308) places the copy and takes the navi's parts: EXE6's
-- a pixel lower down the field and higher, the parts stepping on (its overlay's Param3 and flags 0x14); EXE5's a pixel
-- up the field and lower, a player navi's parts by its NaviStats (0x0800F0EC, `sub_8011420` with r2 1).

local glow = {}

-- Its variants (the original's first parameter, `off_80C4930`'s first and third).
export type Variant = "bug_fix" | "log_in"

export type State = {
    variant: Variant,
    -- The object whose `busy` it clears when done (the original's pointer, its ExtraVars+0).
    holder: Object?,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- A flash, the log in.
    sounds: { flash: Sound, log_in: Sound },
    -- Pixels down the field and up its copy sits from the navi (EXE6's 1, EXE5's -1).
    offset: number,
    -- EXE5's: the parts are a player navi's alone, stepping even while paused (`add_parts_of`'s `paused_stepping`);
    -- EXE6's: any holder's, stepping on (its `keep_stepping`).
    exe5_parts: boolean,
}

export type Glow = {
    kind: Kind,
    -- `sub_80C4AEC`: a glow of `variant` over `spawner`'s side's navi, on its side and facing; `holder` (an object
    -- with a boolean `busy`) is busy until it is done. (It is spawned at the spawner's registers; its init places
    -- it on the navi at once.)
    spawn: (spawner: Object, variant: Variant, holder: Object) -> Object?,
}

-- Variant bug_fix: the flashes' ticks, then the wait.
local FLASH_TICKS, FIRST_FLASH, WAIT = 0x78, 2, 0x1E

-- The navi it copies (its first related object: the side's player).
local function navi_of(me: Object, routine: string): Object
    local navi = me.related1
    if not navi then
        error(string.format("the glow has no navi (%s reads through a null pointer)", routine))
    end
    return navi
end

-- Done: the navi's parts come off it (`sub_8011044` with the navi's
-- record), the navi is back (`sub_80E13DC`), its holder isn't busy, and it
-- goes.
local function leave(me: Object)
    me.visible = false
    local navi = navi_of(me, "sub_8011044")
    me:remove_parts_of(navi)
    dimming.show_user(navi)
    local holder = (me.state :: State).holder
    if holder then
        (holder.state :: { busy: boolean }).busy = false
    end
    me:set_lifecycle("destroy")
end

function glow.make(look: Look): Glow
    -- `sub_80C4958`: the navi vanishes (sparing its barrier visual, status
    -- visuals and the HUD); then a flash whenever Timer2 runs out, Timer2
    -- starting again at a quarter of the ticks so far; at 120 ticks, on.
    local function bug_fix_flashes(me: Object)
        me.sprite.white = false
        if me.phase_init == 0 then
            dimming.hide_user_sparing(navi_of(me, "sub_80E1352"))
            me.timer = 0
            me.timer2 = FIRST_FLASH
            me.phase_init = 4
        end
        local t2 = me.timer2 - 1
        me.timer2 = t2
        if t2 < 0 then
            me.timer2 = bit32.rshift(me.timer, 2)
            me.sprite.white = true
            battle.play_sound(look.sounds.flash)
        end
        local t = me.timer + 1
        me.timer = t
        if t >= FLASH_TICKS then
            me:set_action(4)
        end
    end

    -- `sub_80C49A4` (and variant log_in's second action, `sub_80C4AAC`): 30
    -- ticks, then leave.
    local function wait_and_leave(me: Object)
        if me.phase_init == 0 then
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            leave(me)
        end
    end

    -- `sub_80C4A6C`: the navi vanishes, with the log-in sound; white for 30
    -- ticks.
    local function log_in(me: Object)
        me.sprite.white = false
        if me.phase_init == 0 then
            dimming.hide_user_sparing(navi_of(me, "sub_80E1352"))
            me.timer = WAIT
            battle.play_sound(look.sounds.log_in)
            battle.play_sound(look.sounds.flash)
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            me:set_action(4)
            return
        end
        me.sprite.white = true
    end

    -- `off_80C4930`, then the variant's own action table.
    local function act(me: Object)
        local v = (me.state :: State).variant
        local a = me.action
        if a == 0 then
            if v == "bug_fix" then
                bug_fix_flashes(me)
            else
                log_in(me)
            end
        elseif a == 4 then
            wait_and_leave(me)
        else
            error(string.format("glow action %#x reads past its variant's table", a))
        end
    end

    -- `sub_80C48FC`: seen unless the viewer is blind to its side; its variant;
    -- the sprite, while dimmed too.
    local function tick(me: Object)
        me.visible = true
        me:hide_from_blind()
        act(me)
        me:update_sprite_while_dimmed()
    end

    -- `sub_80C4848`: a copy of the navi: its sprite, a pixel lower down the
    -- field and higher, with its palette and parts; the first update at once.
    local function init(me: Object)
        local navi = navi_of(me, "sub_80C4848")
        local sprite = me.sprite
        sprite:load_look_of(navi)
        sprite.shadow = "ground"
        me.visible = true
        local p = navi.pos
        me.pos = Vec3.new(p.x, p.y + look.offset * 0x10000, p.z + look.offset * 0x10000)
        -- sub_801002C: the navi's palette (the one it wears).
        sprite.palette = navi.sprite.palette
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        if not look.exe5_parts then
            me:add_parts_of(navi, true)
        elseif navi.actor_type == "player" then
            me:add_parts_of(navi, false, true)
        end
        me.anim, me.anim_loaded = 0, 0xFF
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "actor",
        state = { variant = { "bug_fix", "log_in" }, holder = "object" },
        -- `sub_80C4828`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
        end,
    }

    local function spawn(spawner: Object, variant: Variant, holder: Object): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.variant = variant
        s.holder = holder;
        (holder.state :: { busy: boolean }).busy = true
        o.alliance, o.flip = spawner.alliance, spawner.flip
        o.related1 = battle.player(spawner.alliance)
        return o
    end

    return { kind = kind, spawn = spawn }
end

return glow
