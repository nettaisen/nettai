--!strict
-- The engine's test navis: MegaMan with his base form, a link navi with an
-- actor record of its own (AI index 4), and one with hooks of its own (AI
-- index 5), each with the EXE6 overlay's buster and charged shot.

local buster = require("./megaman/weapons/buster")
local charged_shot = require("./megaman/weapons/charged-shot")

-- A sprite's attach points: the blade's point and the gun's (EXE6's gun
-- sits at point 0x0D) ahead of the rest.
local function attach_points(): { { number } }
    local points = {}
    for i = 1, 34 do
        points[i] = { 4, 24 }
    end
    points[4] = { 20, 16 }
    points[0x0E] = { 20, 16 }
    return points
end

local WEAPONS: FormWeapons = { buster = buster.weapon, charge_shot = charged_shot.weapon }

-- What the navi with hooks starts as it lands: three ticks, then out of the
-- air and back to idle with the animation it has.
local landing = define.action {
    id = "test/landing",
    state = { timer = "u16" },
    update = function(me: Object, s: { timer: number })
        if me.step_init == 0 then
            me.step_init = 4
            s.timer = 3
            return
        end
        s.timer -= 1
        if s.timer <= 0 then
            me:set_navi_state("hovering", false)
            me:end_attack()
        end
    end,
}

-- The hooks of the navi that has them, as a navi's own go (EXE5's GyroMan's
-- over a panel that isn't solid; here the top row stands for one): its
-- post-init marks it (it dives); its tick puts it in the air in the top row
-- and asks to take off, and out of the top row asks to land; its idle
-- answers a take-off with nothing more, and a landing with its action.
type Hooks = { tick: (navi: Object) -> (), idle: (navi: Object) -> (), post_init: (navi: Object) -> () }
local HOOKS: Hooks = {
    post_init = function(navi: Object)
        navi:set_navi_state("dives", true)
    end,
    tick = function(navi: Object)
        local hovering = navi:navi_state("hovering")
        if navi.panel_y == 1 and not hovering then
            navi:set_navi_state("hovering", true)
            navi:set_request("take_off", true)
        elseif navi.panel_y ~= 1 and hovering then
            navi:set_request("land", true)
        end
    end,
    idle = function(navi: Object)
        if navi:request("take_off") then
            navi:set_request("take_off", false)
        elseif navi:request("land") then
            navi:set_request("land", false)
            navi:set_attack(landing, 0)
        end
    end,
}

-- `megaman`: the navi that changes form (it has forms, though this content
-- defines none but the base form).
local function navi(id: string, sprite: string, class: IdentityClass, ai_index: number, megaman: boolean, hooks: Hooks?): NaviDef
    return define.navi {
        tick = if hooks then hooks.tick else nil,
        idle = if hooks then hooks.idle else nil,
        post_init = if hooks then hooks.post_init else nil,
        -- (The navi with hooks: its charge glow shows 16 pixels higher in its animation 9.)
        charge_glow_lift = if hooks then { anims = { 9 }, pixels = 16 } else nil,
        id = id,
        sprite = asset.sprite(sprite),
        element = "null",
        buster_bonus = 1,
        move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
        banners = { win = asset.banner("test-win"), lose = asset.banner("test-deleted") },
        merge_height = 0,
        run_message = {},
        weapons = WEAPONS,
        -- What a navi switch brings it with.
        fresh = { hp = 100, mega_level = 5, giga_level = 1 },
        cross_hp = { 100, 150 },
        forms = if megaman then { gregar = { crosses = {} }, falzar = { crosses = {} } } else nil,
        identity = define.identity {
            class = class,
            version = 0,
            actor_type = "player",
            ai_index = ai_index,
            aura_anim = ai_index,
            ice = "medium",
            overlay_hooks = if megaman then { "death", "refresh", "flinch", "drag" } else nil,
            attach_points = attach_points(),
        },
    }
end

return {
    megaman = navi("megaman", "test-megaman", "megaman", 0, true),
    link_navi = navi("test/link-navi", "test-navi", "link_navi", 4, false),
    hooked_navi = navi("test/hooked-navi", "test-navi", "link_navi", 5, false, HOOKS),
    base = define.form {
        id = "base",
        base = true,
        sprite = asset.sprite("test-navi"),
        element = "null",
        buster_bonus = 0,
        weapons = WEAPONS,
        buster_arm = { anim = 0 },
    },
}
