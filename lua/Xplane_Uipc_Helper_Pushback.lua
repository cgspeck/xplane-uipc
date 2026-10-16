-- xplane-uipc pushback helper (FlyWithLua NG / NG+)
--
-- Serves FSUIPC pushback offsets 0x31F0 (status) and 0x31F4 (control):
--   3 = off, 0 = pushing back straight,
--   1 = pushing back, tail to swing left (port),
--   2 = pushing back, tail to swing right (starboard)
--
-- xplane-uipc runs one of the commands below when a client writes 0x31F4, and
-- serves xplane-uipc-helper/pushback/status as 0x31F0.
--
--   xplane-uipc-helper/pushback/straight     (0)
--   xplane-uipc-helper/pushback/tail_left    (1)
--   xplane-uipc-helper/pushback/tail_right   (2)
--   xplane-uipc-helper/pushback/stop         (3)
--
-- With Better Pushback Mod (BPM) installed, a push is a BPM manual push without
-- the yoke. BPM steers the tug in steps: each manual_push_left/right press moves
-- the steer angle 10% of its range, and BPM resets it to centre until the tug is
-- connected and pushing. So the script remembers the requested direction and
-- sends the presses once the aircraft starts moving.
--
-- Without BPM, it falls back to X-Plane's own tug, which only pushes straight:
-- 1 and 2 push straight and the status reads 0.
--
-- A push only starts with the aircraft on the ground and stopped; other start
-- requests are ignored and logged once.

-- ── Settings ──────────────────────────────────────────────────────
-- Steer presses for a turning push (1..10; 10 is BPM's full lock).
local STEER_PRESSES = 5
-- BPM commands that swing the tail left and right. Swap them if the tail goes
-- the wrong way.
local TAIL_LEFT_CMD  = "BetterPushback/manual_push_left"
local TAIL_RIGHT_CMD = "BetterPushback/manual_push_right"
-- Seconds to wait for bp/started after asking BPM to start before giving up.
local START_TIMEOUT = 5
-- Ground speed (m/s) that shows BPM has moved past its starting step.
local MOVING_SPEED = 0.2
-- A push only starts below this ground speed (m/s), as BPM requires.
local START_MAX_SPEED = 1

-- ── Constants ─────────────────────────────────────────────────────
local OFF = 3
local STRAIGHT = 0
local TAIL_LEFT = 1
local TAIL_RIGHT = 2

DataRef("xplane_uipc_pb_groundspeed", "sim/flightmodel/position/groundspeed")
DataRef("xplane_uipc_pb_onground", "sim/flightmodel/failures/onground_any")

define_shared_DataRef("xplane-uipc-helper/pushback/available", "Int")
define_shared_DataRef("xplane-uipc-helper/pushback/status", "Int")
define_shared_DataRef("xplane-uipc-helper/pushback/bpm", "Int")

-- ── State ─────────────────────────────────────────────────────────
local status = OFF            -- what 0x31F0 reports
local bp_started_dr = nil     -- BPM datarefs, found lazily as BPM may load later
local bp_connected_dr = nil
local xp_attached_dr = XPLMFindDataRef("sim/aircraft/overflow/pushback_attached")
local was_started = false     -- bp/started (or pushback_attached) last update
local our_push = false        -- this script started the current BPM push
local stopping = false        -- stop sent, waiting for BPM to finish
local start_requested_at = nil
local steering_live = false   -- BPM keeps steer presses from now on
local steer_applied = 0       -- net presses sent: + = tail left, - = tail right
local refusal_logged = false  -- a refused start has been logged

local function log(msg)
    logMsg("xplane-uipc pushback: " .. msg)
end

-- A start is only sent on the ground and (nearly) stopped. BPM refuses
-- otherwise, and a refused manual push leaves BPM unable to start one until
-- the aircraft or airport is reloaded.
local function can_start()
    if xplane_uipc_pb_onground ~= 1
        or math.abs(xplane_uipc_pb_groundspeed) >= START_MAX_SPEED then
        if not refusal_logged then
            log("ignoring pushback start: aircraft not on the ground and stopped")
            refusal_logged = true
        end
        return false
    end
    refusal_logged = false
    return true
end

local function find_bpm()
    if bp_started_dr == nil then
        bp_started_dr = XPLMFindDataRef("bp/started")
        bp_connected_dr = XPLMFindDataRef("bp/connected")
        if bp_started_dr ~= nil then
            log("Better Pushback Mod found")
        end
    end
    return bp_started_dr ~= nil
end

local function read_bool(dr)
    return dr ~= nil and XPLMGetDatai(dr) ~= 0
end

local function publish()
    set("xplane-uipc-helper/pushback/status", status)
end

local function steer_target(value)
    if value == TAIL_LEFT then
        return STEER_PRESSES
    elseif value == TAIL_RIGHT then
        return -STEER_PRESSES
    end
    return 0
end

local function reset_push()
    our_push = false
    stopping = false
    start_requested_at = nil
    steering_live = false
    steer_applied = 0
end

local function apply_steering()
    local diff = steer_target(status) - steer_applied
    local cmd = diff > 0 and TAIL_LEFT_CMD or TAIL_RIGHT_CMD
    for _ = 1, math.abs(diff) do
        command_once(cmd)
    end
    steer_applied = steer_applied + diff
end

-- ── Requests ──────────────────────────────────────────────────────
local function bpm_request(value)
    local started = read_bool(bp_started_dr)
    if value == OFF then
        if started and not stopping then
            command_once("BetterPushback/stop")
            stopping = true
        end
        status = OFF
        return
    end

    if stopping then
        log("ignoring request " .. value .. " while the last push finishes")
        return
    end

    if not started then
        if not can_start() then
            return
        end
        -- manual_push_start_no_yoke pauses a running manual push, so only
        -- send it when nothing is running.
        reset_push()
        command_once("BetterPushback/manual_push_start_no_yoke")
        our_push = true
        start_requested_at = os.time()
    end
    status = value
    if steering_live then
        apply_steering()
    end
end

local function default_request(value)
    if value == OFF then
        command_once("sim/ground_ops/pushback_stop")
        status = OFF
    elseif status == OFF then
        if can_start() then
            command_once("sim/ground_ops/pushback_straight")
            status = STRAIGHT
        end
    end
end

function xplane_uipc_pushback_request(value)
    if find_bpm() then
        bpm_request(value)
    else
        default_request(value)
    end
    publish()
end

-- ── Tracking ──────────────────────────────────────────────────────
-- Follows pushes started or ended outside xplane-uipc, and starts steering
-- once BPM is pushing.
function xplane_uipc_pushback_update()
    local bpm = find_bpm()
    set("xplane-uipc-helper/pushback/bpm", bpm and 1 or 0)

    local started
    if bpm then
        started = read_bool(bp_started_dr)
    else
        started = read_bool(xp_attached_dr)
    end

    if started and not was_started then
        start_requested_at = nil
        if status == OFF and not stopping then
            status = STRAIGHT  -- started from the BPM or X-Plane menu
        end
    elseif not started and was_started then
        status = OFF
        reset_push()
    elseif bpm and not started and start_requested_at ~= nil
        and os.time() - start_requested_at > START_TIMEOUT then
        log("BPM didn't start the push; reporting off")
        status = OFF
        reset_push()
    end
    was_started = started

    if bpm and our_push and started and not stopping then
        if not steering_live and read_bool(bp_connected_dr)
            and math.abs(xplane_uipc_pb_groundspeed) > MOVING_SPEED then
            steering_live = true
        end
        if steering_live then
            apply_steering()
        end
    end

    publish()
end

create_command("xplane-uipc-helper/pushback/straight",
    "xplane-uipc: push back straight (FSUIPC 0x31F4 = 0)",
    "xplane_uipc_pushback_request(0)", "", "")
create_command("xplane-uipc-helper/pushback/tail_left",
    "xplane-uipc: push back, tail swings left (FSUIPC 0x31F4 = 1)",
    "xplane_uipc_pushback_request(1)", "", "")
create_command("xplane-uipc-helper/pushback/tail_right",
    "xplane-uipc: push back, tail swings right (FSUIPC 0x31F4 = 2)",
    "xplane_uipc_pushback_request(2)", "", "")
create_command("xplane-uipc-helper/pushback/stop",
    "xplane-uipc: stop pushback (FSUIPC 0x31F4 = 3)",
    "xplane_uipc_pushback_request(3)", "", "")

set("xplane-uipc-helper/pushback/available", 1)
publish()

do_often("xplane_uipc_pushback_update()")
