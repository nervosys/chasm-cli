// Copyright (c) 2024-2028 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! SDK Module
//!
//! Provides SDK generation and developer tooling for custom integrations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// SDK Configuration
// ============================================================================

/// SDK configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkConfig {
    /// SDK version
    pub version: String,
    /// Base API URL
    pub base_url: String,
    /// Supported languages
    pub languages: Vec<SdkLanguage>,
    /// API version
    pub api_version: String,
}

/// SDK language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SdkLanguage {
    Python,
    NodeJs,
    Go,
    Rust,
    Java,
    CSharp,
    Ruby,
    Php,
}

impl std::fmt::Display for SdkLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SdkLanguage::Python => write!(f, "python"),
            SdkLanguage::NodeJs => write!(f, "nodejs"),
            SdkLanguage::Go => write!(f, "go"),
            SdkLanguage::Rust => write!(f, "rust"),
            SdkLanguage::Java => write!(f, "java"),
            SdkLanguage::CSharp => write!(f, "csharp"),
            SdkLanguage::Ruby => write!(f, "ruby"),
            SdkLanguage::Php => write!(f, "php"),
        }
    }
}

// ============================================================================
// SDK Templates
// ============================================================================

/// Python SDK template
pub const PYTHON_SDK_TEMPLATE: &str = r#"# Chasm Python SDK
# Auto-generated - Do not edit directly
# Version: {{version}}

"""
Chasm Python SDK

A Python client library for the Chasm API.

Usage:
    from chasm import ChasmClient
    
    client = ChasmClient(api_key="your-api-key")
    sessions = client.sessions.list()
"""

import os
import json
import requests
from typing import Optional, List, Dict, Any, Union
from dataclasses import dataclass, field
from datetime import datetime
from urllib.parse import urljoin

__version__ = "{{version}}"
__api_version__ = "{{api_version}}"


@dataclass
class ChasmConfig:
    """Configuration for Chasm client."""
    base_url: str = "{{base_url}}"
    api_key: Optional[str] = None
    timeout: int = 30
    retry_count: int = 3
    retry_delay: float = 1.0


@dataclass
class Session:
    """Represents a chat session."""
    id: str
    title: str
    provider: str
    workspace_id: Optional[str] = None
    message_count: int = 0
    token_count: int = 0
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None
    tags: List[str] = field(default_factory=list)
    archived: bool = False


@dataclass
class Message:
    """Represents a chat message."""
    id: str
    session_id: str
    role: str
    content: str
    model: Optional[str] = None
    token_count: int = 0
    created_at: Optional[datetime] = None


@dataclass
class Workspace:
    """Represents a workspace."""
    id: str
    name: str
    path: str
    provider: str
    session_count: int = 0
    created_at: Optional[datetime] = None


class ChasmError(Exception):
    """Base exception for Chasm errors."""
    def __init__(self, message: str, status_code: Optional[int] = None, response: Optional[dict] = None):
        super().__init__(message)
        self.status_code = status_code
        self.response = response


class AuthenticationError(ChasmError):
    """Authentication failed."""
    pass


class RateLimitError(ChasmError):
    """Rate limit exceeded."""
    pass


class NotFoundError(ChasmError):
    """Resource not found."""
    pass


class ApiClient:
    """Low-level API client."""
    
    def __init__(self, config: ChasmConfig):
        self.config = config
        self.session = requests.Session()
        if config.api_key:
            self.session.headers["Authorization"] = f"Bearer {config.api_key}"
        self.session.headers["Content-Type"] = "application/json"
        self.session.headers["User-Agent"] = f"chasm-python/{__version__}"
    
    def request(self, method: str, path: str, **kwargs) -> dict:
        """Make an API request."""
        url = urljoin(self.config.base_url, path)
        kwargs.setdefault("timeout", self.config.timeout)
        
        response = self.session.request(method, url, **kwargs)
        
        if response.status_code == 401:
            raise AuthenticationError("Invalid API key", 401)
        elif response.status_code == 404:
            raise NotFoundError("Resource not found", 404)
        elif response.status_code == 429:
            raise RateLimitError("Rate limit exceeded", 429)
        elif response.status_code >= 400:
            raise ChasmError(f"API error: {response.text}", response.status_code)
        
        if response.content:
            return response.json()
        return {}
    
    def get(self, path: str, params: Optional[dict] = None) -> dict:
        return self.request("GET", path, params=params)
    
    def post(self, path: str, data: Optional[dict] = None) -> dict:
        return self.request("POST", path, json=data)
    
    def put(self, path: str, data: Optional[dict] = None) -> dict:
        return self.request("PUT", path, json=data)
    
    def delete(self, path: str) -> dict:
        return self.request("DELETE", path)


class SessionsResource:
    """Sessions API resource."""
    
    def __init__(self, client: ApiClient):
        self._client = client
    
    def list(
        self,
        workspace_id: Optional[str] = None,
        provider: Optional[str] = None,
        archived: Optional[bool] = None,
        limit: int = 20,
        offset: int = 0,
    ) -> List[Session]:
        """List sessions."""
        params = {"limit": limit, "offset": offset}
        if workspace_id:
            params["workspace_id"] = workspace_id
        if provider:
            params["provider"] = provider
        if archived is not None:
            params["archived"] = str(archived).lower()
        
        response = self._client.get("/api/sessions", params)
        return [self._parse_session(s) for s in response.get("sessions", [])]
    
    def get(self, session_id: str) -> Session:
        """Get a session by ID."""
        response = self._client.get(f"/api/sessions/{session_id}")
        return self._parse_session(response)
    
    def create(self, title: str, provider: str, workspace_id: Optional[str] = None) -> Session:
        """Create a new session."""
        data = {"title": title, "provider": provider}
        if workspace_id:
            data["workspace_id"] = workspace_id
        response = self._client.post("/api/sessions", data)
        return self._parse_session(response)
    
    def update(self, session_id: str, **kwargs) -> Session:
        """Update a session."""
        response = self._client.put(f"/api/sessions/{session_id}", kwargs)
        return self._parse_session(response)
    
    def delete(self, session_id: str) -> bool:
        """Delete a session."""
        self._client.delete(f"/api/sessions/{session_id}")
        return True
    
    def archive(self, session_id: str) -> Session:
        """Archive a session."""
        return self.update(session_id, archived=True)
    
    def search(self, query: str, limit: int = 20) -> List[Session]:
        """Search sessions."""
        response = self._client.get("/api/sessions/search", {"q": query, "limit": limit})
        return [self._parse_session(s) for s in response.get("sessions", [])]
    
    def _parse_session(self, data: dict) -> Session:
        return Session(
            id=data["id"],
            title=data.get("title", "Untitled"),
            provider=data.get("provider", "unknown"),
            workspace_id=data.get("workspace_id"),
            message_count=data.get("message_count", 0),
            token_count=data.get("token_count", 0),
            tags=data.get("tags", []),
            archived=data.get("archived", False),
        )


class WorkspacesResource:
    """Workspaces API resource."""
    
    def __init__(self, client: ApiClient):
        self._client = client
    
    def list(self, limit: int = 20, offset: int = 0) -> List[Workspace]:
        """List workspaces."""
        response = self._client.get("/api/workspaces", {"limit": limit, "offset": offset})
        return [self._parse_workspace(w) for w in response.get("workspaces", [])]
    
    def get(self, workspace_id: str) -> Workspace:
        """Get a workspace by ID."""
        response = self._client.get(f"/api/workspaces/{workspace_id}")
        return self._parse_workspace(response)
    
    def _parse_workspace(self, data: dict) -> Workspace:
        return Workspace(
            id=data["id"],
            name=data.get("name", ""),
            path=data.get("path", ""),
            provider=data.get("provider", ""),
            session_count=data.get("session_count", 0),
        )


class HarvestResource:
    """Harvest API resource."""
    
    def __init__(self, client: ApiClient):
        self._client = client
    
    def run(self, providers: Optional[List[str]] = None) -> dict:
        """Run harvest."""
        data = {}
        if providers:
            data["providers"] = providers
        return self._client.post("/api/harvest", data)
    
    def status(self) -> dict:
        """Get harvest status."""
        return self._client.get("/api/harvest/status")


class ChasmClient:
    """Main Chasm client."""
    
    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        **kwargs
    ):
        api_key = api_key or os.environ.get("CHASM_API_KEY")
        config = ChasmConfig(
            api_key=api_key,
            base_url=base_url or "{{base_url}}",
            **kwargs
        )
        self._api = ApiClient(config)
        
        # Resources
        self.sessions = SessionsResource(self._api)
        self.workspaces = WorkspacesResource(self._api)
        self.harvest = HarvestResource(self._api)
    
    def health(self) -> dict:
        """Check API health."""
        return self._api.get("/health")
    
    def stats(self) -> dict:
        """Get statistics."""
        return self._api.get("/api/stats")


# Convenience function
def create_client(**kwargs) -> ChasmClient:
    """Create a Chasm client with environment configuration."""
    return ChasmClient(**kwargs)
"#;

/// Node.js SDK template
pub const NODEJS_SDK_TEMPLATE: &str = r#"/**
 * Chasm Node.js SDK
 * Auto-generated - Do not edit directly
 * Version: {{version}}
 */

const https = require('https');
const http = require('http');
const { URL } = require('url');

const VERSION = '{{version}}';
const API_VERSION = '{{api_version}}';

/**
 * Chasm client configuration
 */
class ChasmConfig {
  constructor(options = {}) {
    this.baseUrl = options.baseUrl || process.env.CHASM_BASE_URL || '{{base_url}}';
    this.apiKey = options.apiKey || process.env.CHASM_API_KEY;
    this.timeout = options.timeout || 30000;
    this.retryCount = options.retryCount || 3;
  }
}

/**
 * Custom error classes
 */
class ChasmError extends Error {
  constructor(message, statusCode, response) {
    super(message);
    this.name = 'ChasmError';
    this.statusCode = statusCode;
    this.response = response;
  }
}

class AuthenticationError extends ChasmError {
  constructor(message) {
    super(message, 401);
    this.name = 'AuthenticationError';
  }
}

class NotFoundError extends ChasmError {
  constructor(message) {
    super(message, 404);
    this.name = 'NotFoundError';
  }
}

class RateLimitError extends ChasmError {
  constructor(message) {
    super(message, 429);
    this.name = 'RateLimitError';
  }
}

/**
 * Low-level API client
 */
class ApiClient {
  constructor(config) {
    this.config = config;
  }

  async request(method, path, options = {}) {
    const url = new URL(path, this.config.baseUrl);
    const isHttps = url.protocol === 'https:';
    const client = isHttps ? https : http;

    if (options.params) {
      Object.entries(options.params).forEach(([key, value]) => {
        if (value !== undefined) {
          url.searchParams.append(key, String(value));
        }
      });
    }

    const requestOptions = {
      method,
      hostname: url.hostname,
      port: url.port || (isHttps ? 443 : 80),
      path: url.pathname + url.search,
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': `chasm-nodejs/${VERSION}`,
        ...(this.config.apiKey && { Authorization: `Bearer ${this.config.apiKey}` }),
      },
      timeout: this.config.timeout,
    };

    return new Promise((resolve, reject) => {
      const req = client.request(requestOptions, (res) => {
        let data = '';
        res.on('data', (chunk) => (data += chunk));
        res.on('end', () => {
          if (res.statusCode === 401) {
            reject(new AuthenticationError('Invalid API key'));
          } else if (res.statusCode === 404) {
            reject(new NotFoundError('Resource not found'));
          } else if (res.statusCode === 429) {
            reject(new RateLimitError('Rate limit exceeded'));
          } else if (res.statusCode >= 400) {
            reject(new ChasmError(`API error: ${data}`, res.statusCode));
          } else {
            resolve(data ? JSON.parse(data) : {});
          }
        });
      });

      req.on('error', reject);
      req.on('timeout', () => {
        req.destroy();
        reject(new ChasmError('Request timeout'));
      });

      if (options.body) {
        req.write(JSON.stringify(options.body));
      }
      req.end();
    });
  }

  get(path, params) {
    return this.request('GET', path, { params });
  }

  post(path, body) {
    return this.request('POST', path, { body });
  }

  put(path, body) {
    return this.request('PUT', path, { body });
  }

  delete(path) {
    return this.request('DELETE', path);
  }
}

/**
 * Sessions resource
 */
class SessionsResource {
  constructor(client) {
    this._client = client;
  }

  async list(options = {}) {
    const params = {
      limit: options.limit || 20,
      offset: options.offset || 0,
      workspace_id: options.workspaceId,
      provider: options.provider,
      archived: options.archived,
    };
    const response = await this._client.get('/api/sessions', params);
    return response.sessions || [];
  }

  async get(sessionId) {
    return this._client.get(`/api/sessions/${sessionId}`);
  }

  async create(data) {
    return this._client.post('/api/sessions', data);
  }

  async update(sessionId, data) {
    return this._client.put(`/api/sessions/${sessionId}`, data);
  }

  async delete(sessionId) {
    await this._client.delete(`/api/sessions/${sessionId}`);
    return true;
  }

  async search(query, limit = 20) {
    const response = await this._client.get('/api/sessions/search', { q: query, limit });
    return response.sessions || [];
  }
}

/**
 * Workspaces resource
 */
class WorkspacesResource {
  constructor(client) {
    this._client = client;
  }

  async list(options = {}) {
    const params = {
      limit: options.limit || 20,
      offset: options.offset || 0,
    };
    const response = await this._client.get('/api/workspaces', params);
    return response.workspaces || [];
  }

  async get(workspaceId) {
    return this._client.get(`/api/workspaces/${workspaceId}`);
  }
}

/**
 * Harvest resource
 */
class HarvestResource {
  constructor(client) {
    this._client = client;
  }

  async run(providers) {
    const data = providers ? { providers } : {};
    return this._client.post('/api/harvest', data);
  }

  async status() {
    return this._client.get('/api/harvest/status');
  }
}

/**
 * Main Chasm client
 */
class ChasmClient {
  constructor(options = {}) {
    const config = new ChasmConfig(options);
    this._api = new ApiClient(config);

    this.sessions = new SessionsResource(this._api);
    this.workspaces = new WorkspacesResource(this._api);
    this.harvest = new HarvestResource(this._api);
  }

  async health() {
    return this._api.get('/health');
  }

  async stats() {
    return this._api.get('/api/stats');
  }
}

module.exports = {
  ChasmClient,
  ChasmConfig,
  ChasmError,
  AuthenticationError,
  NotFoundError,
  RateLimitError,
  VERSION,
  API_VERSION,
};
"#;

/// Go SDK template
pub const GO_SDK_TEMPLATE: &str = r#"// Chasm Go SDK
// Auto-generated - Do not edit directly
// Version: {{version}}

package chasm

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"time"
)

const (
	Version    = "{{version}}"
	APIVersion = "{{api_version}}"
)

// Config holds client configuration
type Config struct {
	BaseURL    string
	APIKey     string
	Timeout    time.Duration
	RetryCount int
}

// DefaultConfig returns default configuration
func DefaultConfig() *Config {
	baseURL := os.Getenv("CHASM_BASE_URL")
	if baseURL == "" {
		baseURL = "{{base_url}}"
	}
	return &Config{
		BaseURL:    baseURL,
		APIKey:     os.Getenv("CHASM_API_KEY"),
		Timeout:    30 * time.Second,
		RetryCount: 3,
	}
}

// Session represents a chat session
type Session struct {
	ID           string    `json:"id"`
	Title        string    `json:"title"`
	Provider     string    `json:"provider"`
	WorkspaceID  *string   `json:"workspace_id,omitempty"`
	MessageCount int       `json:"message_count"`
	TokenCount   int       `json:"token_count"`
	Tags         []string  `json:"tags"`
	Archived     bool      `json:"archived"`
	CreatedAt    time.Time `json:"created_at,omitempty"`
	UpdatedAt    time.Time `json:"updated_at,omitempty"`
}

// Workspace represents a workspace
type Workspace struct {
	ID           string    `json:"id"`
	Name         string    `json:"name"`
	Path         string    `json:"path"`
	Provider     string    `json:"provider"`
	SessionCount int       `json:"session_count"`
	CreatedAt    time.Time `json:"created_at,omitempty"`
}

// Error types
type ChasmError struct {
	Message    string
	StatusCode int
}

func (e *ChasmError) Error() string {
	return fmt.Sprintf("chasm: %s (status %d)", e.Message, e.StatusCode)
}

// Client is the main Chasm client
type Client struct {
	config     *Config
	httpClient *http.Client
	Sessions   *SessionsService
	Workspaces *WorkspacesService
	Harvest    *HarvestService
}

// NewClient creates a new Chasm client
func NewClient(config *Config) *Client {
	if config == nil {
		config = DefaultConfig()
	}
	
	c := &Client{
		config: config,
		httpClient: &http.Client{
			Timeout: config.Timeout,
		},
	}
	
	c.Sessions = &SessionsService{client: c}
	c.Workspaces = &WorkspacesService{client: c}
	c.Harvest = &HarvestService{client: c}
	
	return c
}

func (c *Client) request(method, path string, body interface{}, result interface{}) error {
	u, err := url.Parse(c.config.BaseURL + path)
	if err != nil {
		return err
	}

	var bodyReader io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return err
		}
		bodyReader = bytes.NewReader(b)
	}

	req, err := http.NewRequest(method, u.String(), bodyReader)
	if err != nil {
		return err
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("User-Agent", fmt.Sprintf("chasm-go/%s", Version))
	if c.config.APIKey != "" {
		req.Header.Set("Authorization", "Bearer "+c.config.APIKey)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return &ChasmError{
			Message:    fmt.Sprintf("API error: %s", resp.Status),
			StatusCode: resp.StatusCode,
		}
	}

	if result != nil {
		return json.NewDecoder(resp.Body).Decode(result)
	}
	return nil
}

// Health checks API health
func (c *Client) Health() (map[string]interface{}, error) {
	var result map[string]interface{}
	err := c.request("GET", "/health", nil, &result)
	return result, err
}

// Stats gets statistics
func (c *Client) Stats() (map[string]interface{}, error) {
	var result map[string]interface{}
	err := c.request("GET", "/api/stats", nil, &result)
	return result, err
}

// SessionsService handles session operations
type SessionsService struct {
	client *Client
}

// ListOptions for listing resources
type ListOptions struct {
	Limit       int
	Offset      int
	WorkspaceID string
	Provider    string
	Archived    *bool
}

func (s *SessionsService) List(opts *ListOptions) ([]Session, error) {
	path := "/api/sessions"
	if opts != nil {
		params := url.Values{}
		if opts.Limit > 0 {
			params.Set("limit", fmt.Sprintf("%d", opts.Limit))
		}
		if opts.Offset > 0 {
			params.Set("offset", fmt.Sprintf("%d", opts.Offset))
		}
		if opts.WorkspaceID != "" {
			params.Set("workspace_id", opts.WorkspaceID)
		}
		if opts.Provider != "" {
			params.Set("provider", opts.Provider)
		}
		if opts.Archived != nil {
			params.Set("archived", fmt.Sprintf("%t", *opts.Archived))
		}
		if len(params) > 0 {
			path += "?" + params.Encode()
		}
	}
	
	var result struct {
		Sessions []Session `json:"sessions"`
	}
	err := s.client.request("GET", path, nil, &result)
	return result.Sessions, err
}

func (s *SessionsService) Get(id string) (*Session, error) {
	var session Session
	err := s.client.request("GET", "/api/sessions/"+id, nil, &session)
	return &session, err
}

func (s *SessionsService) Create(title, provider string, workspaceID *string) (*Session, error) {
	body := map[string]interface{}{
		"title":    title,
		"provider": provider,
	}
	if workspaceID != nil {
		body["workspace_id"] = *workspaceID
	}
	var session Session
	err := s.client.request("POST", "/api/sessions", body, &session)
	return &session, err
}

func (s *SessionsService) Delete(id string) error {
	return s.client.request("DELETE", "/api/sessions/"+id, nil, nil)
}

func (s *SessionsService) Search(query string, limit int) ([]Session, error) {
	path := fmt.Sprintf("/api/sessions/search?q=%s&limit=%d", url.QueryEscape(query), limit)
	var result struct {
		Sessions []Session `json:"sessions"`
	}
	err := s.client.request("GET", path, nil, &result)
	return result.Sessions, err
}

// WorkspacesService handles workspace operations
type WorkspacesService struct {
	client *Client
}

func (w *WorkspacesService) List(opts *ListOptions) ([]Workspace, error) {
	path := "/api/workspaces"
	if opts != nil && (opts.Limit > 0 || opts.Offset > 0) {
		params := url.Values{}
		if opts.Limit > 0 {
			params.Set("limit", fmt.Sprintf("%d", opts.Limit))
		}
		if opts.Offset > 0 {
			params.Set("offset", fmt.Sprintf("%d", opts.Offset))
		}
		path += "?" + params.Encode()
	}
	
	var result struct {
		Workspaces []Workspace `json:"workspaces"`
	}
	err := w.client.request("GET", path, nil, &result)
	return result.Workspaces, err
}

func (w *WorkspacesService) Get(id string) (*Workspace, error) {
	var workspace Workspace
	err := w.client.request("GET", "/api/workspaces/"+id, nil, &workspace)
	return &workspace, err
}

// HarvestService handles harvest operations
type HarvestService struct {
	client *Client
}

func (h *HarvestService) Run(providers []string) (map[string]interface{}, error) {
	body := map[string]interface{}{}
	if len(providers) > 0 {
		body["providers"] = providers
	}
	var result map[string]interface{}
	err := h.client.request("POST", "/api/harvest", body, &result)
	return result, err
}

func (h *HarvestService) Status() (map[string]interface{}, error) {
	var result map[string]interface{}
	err := h.client.request("GET", "/api/harvest/status", nil, &result)
	return result, err
}
"#;

// ============================================================================
// SDK Generator
// ============================================================================

/// SDK generator
pub struct SdkGenerator {
    config: SdkConfig,
}

impl SdkGenerator {
    /// Create a new SDK generator
    pub fn new(config: SdkConfig) -> Self {
        Self { config }
    }

    /// Generate SDK for a language
    pub fn generate(&self, language: SdkLanguage) -> String {
        let template = match language {
            SdkLanguage::Python => PYTHON_SDK_TEMPLATE,
            SdkLanguage::NodeJs => NODEJS_SDK_TEMPLATE,
            SdkLanguage::Go => GO_SDK_TEMPLATE,
            _ => return format!("// SDK for {} not yet implemented", language),
        };

        template
            .replace("{{version}}", &self.config.version)
            .replace("{{api_version}}", &self.config.api_version)
            .replace("{{base_url}}", &self.config.base_url)
    }

    /// Generate all SDKs
    pub fn generate_all(&self) -> HashMap<SdkLanguage, String> {
        self.config
            .languages
            .iter()
            .map(|lang| (lang.clone(), self.generate(lang.clone())))
            .collect()
    }

    /// Get SDK file name for language
    pub fn get_filename(&self, language: &SdkLanguage) -> String {
        match language {
            SdkLanguage::Python => "chasm.py".to_string(),
            SdkLanguage::NodeJs => "chasm.js".to_string(),
            SdkLanguage::Go => "chasm.go".to_string(),
            SdkLanguage::Rust => "chasm.rs".to_string(),
            SdkLanguage::Java => "Chasm.java".to_string(),
            SdkLanguage::CSharp => "Chasm.cs".to_string(),
            SdkLanguage::Ruby => "chasm.rb".to_string(),
            SdkLanguage::Php => "Chasm.php".to_string(),
        }
    }
}

impl Default for SdkConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            base_url: "http://localhost:8787".to_string(),
            languages: vec![SdkLanguage::Python, SdkLanguage::NodeJs, SdkLanguage::Go],
            api_version: "v1".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdk_generation() {
        let config = SdkConfig::default();
        let generator = SdkGenerator::new(config);

        let python_sdk = generator.generate(SdkLanguage::Python);
        assert!(python_sdk.contains("class ChasmClient"));
        assert!(python_sdk.contains("1.0.0"));

        let nodejs_sdk = generator.generate(SdkLanguage::NodeJs);
        assert!(nodejs_sdk.contains("class ChasmClient"));

        let go_sdk = generator.generate(SdkLanguage::Go);
        assert!(go_sdk.contains("type Client struct"));
    }
}
