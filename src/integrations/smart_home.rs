// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Smart Home Integrations
//!
//! Home Assistant, HomeKit, Hue, SmartThings, IoT devices

use super::IntegrationResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Common Types
// =============================================================================

/// Device state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub device_id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub state: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub last_changed: String,
    pub is_online: bool,
}

/// Device types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Light,
    Switch,
    Outlet,
    Thermostat,
    Lock,
    Sensor,
    Camera,
    Speaker,
    Fan,
    Blind,
    Garage,
    Vacuum,
    AirPurifier,
    Humidifier,
    MediaPlayer,
    Climate,
    Other,
}

/// Scene/Automation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub actions: Vec<DeviceAction>,
    pub icon: Option<String>,
}

/// Device action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAction {
    pub device_id: String,
    pub action: String,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

// =============================================================================
// Home Assistant
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeAssistantConfig {
    pub url: String,
    pub access_token: String,
}

/// Home Assistant entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAEntity {
    pub entity_id: String,
    pub state: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub last_changed: String,
    pub last_updated: String,
}

/// Home Assistant service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAService {
    pub domain: String,
    pub service: String,
    pub description: Option<String>,
    pub fields: HashMap<String, HAServiceField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAServiceField {
    pub description: String,
    pub required: bool,
    pub example: Option<serde_json::Value>,
}

/// Home Assistant automation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HAAutomation {
    pub id: String,
    pub alias: String,
    pub description: Option<String>,
    pub trigger: Vec<serde_json::Value>,
    pub condition: Vec<serde_json::Value>,
    pub action: Vec<serde_json::Value>,
    pub mode: String,
}

/// Home Assistant provider trait
#[async_trait::async_trait]
pub trait HomeAssistantProvider: Send + Sync {
    // States
    async fn list_entities(&self, domain: Option<&str>) -> IntegrationResult;
    async fn get_state(&self, entity_id: &str) -> IntegrationResult;
    async fn get_history(&self, entity_id: &str, start: &str, end: &str) -> IntegrationResult;

    // Services
    async fn list_services(&self) -> IntegrationResult;
    async fn call_service(
        &self,
        domain: &str,
        service: &str,
        data: Option<serde_json::Value>,
    ) -> IntegrationResult;

    // Automation
    async fn list_automations(&self) -> IntegrationResult;
    async fn trigger_automation(&self, automation_id: &str) -> IntegrationResult;
    async fn toggle_automation(&self, automation_id: &str, enabled: bool) -> IntegrationResult;

    // Scenes
    async fn list_scenes(&self) -> IntegrationResult;
    async fn activate_scene(&self, scene_id: &str) -> IntegrationResult;

    // Scripts
    async fn run_script(
        &self,
        script_id: &str,
        data: Option<serde_json::Value>,
    ) -> IntegrationResult;

    // Events
    async fn fire_event(
        &self,
        event_type: &str,
        data: Option<serde_json::Value>,
    ) -> IntegrationResult;

    // Companion app
    async fn notify(
        &self,
        message: &str,
        title: Option<&str>,
        data: Option<serde_json::Value>,
    ) -> IntegrationResult;
}

// =============================================================================
// Apple HomeKit
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeKitAccessory {
    pub id: String,
    pub name: String,
    pub room: Option<String>,
    pub accessory_type: String,
    pub services: Vec<HomeKitService>,
    pub is_reachable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeKitService {
    pub service_type: String,
    pub characteristics: Vec<HomeKitCharacteristic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeKitCharacteristic {
    pub characteristic_type: String,
    pub value: serde_json::Value,
    pub is_readable: bool,
    pub is_writable: bool,
}

/// HomeKit provider trait
#[async_trait::async_trait]
pub trait HomeKitProvider: Send + Sync {
    async fn list_homes(&self) -> IntegrationResult;
    async fn list_rooms(&self, home_id: &str) -> IntegrationResult;
    async fn list_accessories(&self, home_id: &str) -> IntegrationResult;
    async fn get_accessory(&self, accessory_id: &str) -> IntegrationResult;
    async fn set_characteristic(
        &self,
        accessory_id: &str,
        service: &str,
        characteristic: &str,
        value: serde_json::Value,
    ) -> IntegrationResult;
    async fn list_scenes(&self, home_id: &str) -> IntegrationResult;
    async fn run_scene(&self, scene_id: &str) -> IntegrationResult;
}

// =============================================================================
// Philips Hue
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueConfig {
    pub bridge_ip: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueLight {
    pub id: String,
    pub name: String,
    pub is_on: bool,
    pub brightness: u8,          // 1-254
    pub hue: Option<u16>,        // 0-65535
    pub saturation: Option<u8>,  // 0-254
    pub color_temp: Option<u16>, // 153-500 (mirek)
    pub is_reachable: bool,
    pub light_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueGroup {
    pub id: String,
    pub name: String,
    pub lights: Vec<String>,
    pub group_type: String,
    pub is_on: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HueScene {
    pub id: String,
    pub name: String,
    pub group: Option<String>,
    pub lights: Vec<String>,
}

/// Hue provider trait
#[async_trait::async_trait]
pub trait HueProvider: Send + Sync {
    // Lights
    async fn list_lights(&self) -> IntegrationResult;
    async fn get_light(&self, light_id: &str) -> IntegrationResult;
    async fn set_light_state(
        &self,
        light_id: &str,
        on: Option<bool>,
        brightness: Option<u8>,
        color: Option<(u16, u8)>,
    ) -> IntegrationResult;

    // Groups
    async fn list_groups(&self) -> IntegrationResult;
    async fn set_group_state(
        &self,
        group_id: &str,
        on: Option<bool>,
        brightness: Option<u8>,
    ) -> IntegrationResult;

    // Scenes
    async fn list_scenes(&self) -> IntegrationResult;
    async fn activate_scene(&self, scene_id: &str) -> IntegrationResult;

    // Effects
    async fn set_color_loop(&self, light_id: &str, enabled: bool) -> IntegrationResult;
    async fn alert(&self, light_id: &str) -> IntegrationResult;
}

// =============================================================================
// Thermostats (Nest, Ecobee)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermostatState {
    pub id: String,
    pub name: String,
    pub current_temperature: f32,
    pub target_temperature: Option<f32>,
    pub target_temperature_low: Option<f32>,
    pub target_temperature_high: Option<f32>,
    pub mode: ThermostatMode,
    pub humidity: Option<u8>,
    pub is_running: bool,
    pub fan_mode: Option<FanMode>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermostatMode {
    Off,
    Heat,
    Cool,
    HeatCool,
    Auto,
    Eco,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FanMode {
    Auto,
    On,
    Circulate,
}

/// Thermostat provider trait
#[async_trait::async_trait]
pub trait ThermostatProvider: Send + Sync {
    async fn get_state(&self, thermostat_id: &str) -> IntegrationResult;
    async fn set_temperature(&self, thermostat_id: &str, temperature: f32) -> IntegrationResult;
    async fn set_mode(&self, thermostat_id: &str, mode: ThermostatMode) -> IntegrationResult;
    async fn set_fan_mode(&self, thermostat_id: &str, fan_mode: FanMode) -> IntegrationResult;
    async fn set_schedule(
        &self,
        thermostat_id: &str,
        schedule: serde_json::Value,
    ) -> IntegrationResult;
}

// =============================================================================
// Locks (August, Schlage)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockState {
    pub id: String,
    pub name: String,
    pub is_locked: bool,
    pub is_jammed: bool,
    pub battery_level: Option<u8>,
    pub last_activity: Option<String>,
}

/// Lock provider trait
#[async_trait::async_trait]
pub trait LockProvider: Send + Sync {
    async fn get_state(&self, lock_id: &str) -> IntegrationResult;
    async fn lock(&self, lock_id: &str) -> IntegrationResult;
    async fn unlock(&self, lock_id: &str) -> IntegrationResult;
    async fn get_activity(&self, lock_id: &str, limit: u32) -> IntegrationResult;
}

// =============================================================================
// Cameras (Ring, Nest, Wyze)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub id: String,
    pub name: String,
    pub is_online: bool,
    pub is_recording: bool,
    pub has_motion: bool,
    pub battery_level: Option<u8>,
    pub stream_url: Option<String>,
}

/// Camera provider trait
#[async_trait::async_trait]
pub trait CameraProvider: Send + Sync {
    async fn list_cameras(&self) -> IntegrationResult;
    async fn get_camera(&self, camera_id: &str) -> IntegrationResult;
    async fn get_stream_url(&self, camera_id: &str) -> IntegrationResult;
    async fn get_snapshot(&self, camera_id: &str) -> IntegrationResult;
    async fn get_events(&self, camera_id: &str, limit: u32) -> IntegrationResult;
}

// =============================================================================
// Sensors
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    pub id: String,
    pub name: String,
    pub sensor_type: SensorType,
    pub value: serde_json::Value,
    pub unit: Option<String>,
    pub battery_level: Option<u8>,
    pub last_updated: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    Temperature,
    Humidity,
    Motion,
    Contact,
    Light,
    Smoke,
    CarbonMonoxide,
    Water,
    Vibration,
    Pressure,
    AirQuality,
    Power,
    Energy,
}

/// Sensor provider trait
#[async_trait::async_trait]
pub trait SensorProvider: Send + Sync {
    async fn list_sensors(&self) -> IntegrationResult;
    async fn get_sensor(&self, sensor_id: &str) -> IntegrationResult;
    async fn get_history(&self, sensor_id: &str, start: &str, end: &str) -> IntegrationResult;
}
