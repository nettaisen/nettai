--!strict
-- The glow over a navi (the original's actor object #0x5D, `sub_80C4828`;
-- docs/engine/dimming-chips.md §5.2): a copy of the side's navi, which
-- vanishes under it, flashing white. BugFix's controller spawns its first
-- variant; the other two are the original's, which nothing spawns.
--
--   bug_fix (`sub_80C493C`): white flashes further and further apart for
--     120 ticks, then 30 ticks, and the navi is back;
--   submerge (`sub_80C49E4`): white for 31 ticks, then the navi is back,
--     submerged for 480 ticks;
--   log_in (`sub_80C4A52`): white for 30 ticks with the log-in sound, then
--     as bug_fix's end.
--
-- When it is done it clears its spawner's `busy` flag.

local viewer = require("../../lib/viewer")

local glow = {}

-- Its variants (the original's first parameter, `off_80C4930`).
glow.BUG_FIX, glow.SUBMERGE, glow.LOG_IN = 0, 1, 2

export type State = {
    variant: number,
    -- The object whose `busy` it clears when done (the original's pointer,
    -- its ExtraVars+0).
    holder: Object?,
}

-- Sounds: a flash, the log in, going under.
local FLASH_SOUND = asset.sound("bugfix-flash")
local LOG_IN_SOUND = asset.sound("log-in-77")
local SUBMERGE_SOUND = asset.sound("invisible")
-- Variant bug_fix: the flashes' ticks, then the wait.
local FLASH_TICKS, FIRST_FLASH, WAIT = 0x78, 2, 0x1E
-- Variant submerge: submerged for 0xF0 * 2 ticks.
local SUBMERGED_TICKS = 0x1E0

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
        battle.play_sound(FLASH_SOUND)
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

-- `sub_80C49E4`: white; 31 ticks; then the navi is back, submerged.
local function submerge(me: Object)
    if me.phase_init == 0 then
        local navi = navi_of(me, "sub_80E1352")
        dimming.hide_user_sparing(navi)
        -- sub_80101C4: not submerged any more.
        navi:set_status_timer("submerged", 0)
        navi:set_status("submerged", false)
        me.timer = 0
        me.sprite.white = true
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    me.visible = false
    local navi = navi_of(me, "sub_8011044")
    me:remove_parts_of(navi)
    dimming.show_user(navi)
    -- sub_80101AE: submerged for a while, and out of sight.
    navi:set_status_timer("submerged", SUBMERGED_TICKS)
    navi:set_status("submerged", true)
    navi.visible = false
    battle.play_sound(SUBMERGE_SOUND)
    local holder = (me.state :: State).holder
    if holder then
        (holder.state :: { busy: boolean }).busy = false
    end
    me:set_lifecycle("destroy")
end

-- `sub_80C4A6C`: the navi vanishes, with the log-in sound; white for 30
-- ticks.
local function log_in(me: Object)
    me.sprite.white = false
    if me.phase_init == 0 then
        dimming.hide_user_sparing(navi_of(me, "sub_80E1352"))
        me.timer = WAIT
        battle.play_sound(LOG_IN_SOUND)
        battle.play_sound(FLASH_SOUND)
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
    if v == glow.SUBMERGE then
        submerge(me)
    elseif a == 0 then
        if v == glow.BUG_FIX then
            bug_fix_flashes(me)
        elseif v == glow.LOG_IN then
            log_in(me)
        else
            error(string.format("glow variant %d reads past off_80C4930", v))
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
    me.visible = viewer.sees(me.alliance)
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
    me.pos = Vec3.new(p.x, p.y + 0x10000, p.z + 0x10000)
    -- sub_801002C: the navi's palette (the one it wears).
    sprite.palette = navi.sprite.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_parts_of(navi, true)
    me.anim, me.anim_loaded = 0, 0xFF
    me:set_lifecycle("update")
    tick(me)
end

glow.kind = define.kind {
    id = "bugfix/glow",
    pool = "actor",
    state = { variant = "u8", holder = "object" },
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

-- `sub_80C4AEC`: a glow of `variant` over `spawner`'s side's navi, on its
-- side and facing; `holder` (an object with a boolean `busy`) is busy
-- until it is done. (It is spawned at the spawner's registers; its init
-- places it on the navi at once.)
function glow.spawn(spawner: Object, variant: number, holder: Object): Object?
    local o = battle.spawn(glow.kind, Vec3.zero)
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

return glow
