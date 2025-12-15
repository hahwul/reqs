# 소스코드 리팩토링 문서 (Source Code Refactoring Documentation)

## 개요 (Overview)

이 문서는 `reqs` 프로젝트의 소스코드 리팩토링 작업을 설명합니다.
This document describes the source code refactoring work for the `reqs` project.

## 리팩토링 목표 (Refactoring Goals)

1. **모듈화 (Modularization)**: 1,135줄의 단일 파일을 논리적 모듈로 분리
2. **코드 품질 향상 (Code Quality Improvement)**: Clippy 경고 제거 및 베스트 프랙티스 적용
3. **테스트 커버리지 (Test Coverage)**: 핵심 기능에 대한 유닛 테스트 추가
4. **유지보수성 (Maintainability)**: 함수 크기 축소 및 단일 책임 원칙 적용
5. **재사용성 (Reusability)**: 공통 기능을 독립적인 모듈로 추출

## 변경 사항 (Changes)

### 이전 구조 (Before)

```
src/
  main.rs (1,135 lines)
```

- 모든 코드가 단일 파일에 포함
- 높은 결합도
- 테스트 없음
- 긴 함수 (일부 함수가 200줄 이상)

### 새로운 구조 (After)

```
src/
  main.rs (32 lines)          - 진입점 (Entry point)
  constants.rs (13 lines)     - 상수 정의 (Constants)
  types.rs (112 lines)        - 타입 정의 (Type definitions)
  processor.rs (348 lines)    - 요청 처리 (Request processing)
  
  http/
    mod.rs (7 lines)
    client.rs (39 lines)      - HTTP 클라이언트 빌더 (HTTP client builder)
    headers.rs (49 lines)     - 헤더 파싱 (Header parsing)
    request.rs (126 lines)    - 요청 생성 및 포맷팅 (Request building & formatting)
  
  filter/
    mod.rs (90 lines)         - 응답 필터링 (Response filtering)
  
  output/
    mod.rs (3 lines)
    formatter.rs (116 lines)  - 출력 포맷팅 (Output formatting)
  
  utils/
    mod.rs (7 lines)
    url.rs (69 lines)         - URL 정규화 (URL normalization)
    delay.rs (56 lines)       - 지연 및 속도 제한 (Delay & rate limiting)
    html.rs (39 lines)        - HTML 파싱 (HTML parsing)
  
  mcp/
    mod.rs (3 lines)
    server.rs (465 lines)     - MCP 서버 구현 (MCP server implementation)
```

**총 라인 수 (Total Lines)**: 1,574 lines (테스트 포함 / including tests)
**모듈 수 (Number of Modules)**: 17 files
**테스트 수 (Number of Tests)**: 17 unit tests

## 주요 개선 사항 (Key Improvements)

### 1. 모듈 분리 (Module Separation)

#### HTTP 모듈 (`src/http/`)
- **client.rs**: HTTP 클라이언트 생성 로직을 단일 함수로 통합
- **headers.rs**: 헤더 파싱 로직 분리 및 테스트 추가
- **request.rs**: 요청 빌드 및 포맷팅 로직 분리

#### 필터 모듈 (`src/filter/`)
- 응답 필터링 로직을 독립적인 모듈로 분리
- 상태 코드, 문자열, 정규식 필터링 지원
- 테스트로 각 필터 타입 검증

#### 출력 모듈 (`src/output/`)
- Plain, JSON, CSV 출력 포맷팅 로직 분리
- `ResponseInfo` 구조체로 응답 데이터 캡슐화
- 색상 출력 로직 통합

#### 유틸리티 모듈 (`src/utils/`)
- **url.rs**: URL 스키마 정규화 (http/https 자동 추가)
- **delay.rs**: 랜덤 지연 및 속도 제한 기능
- **html.rs**: HTML 제목 추출 기능

#### MCP 모듈 (`src/mcp/`)
- MCP (Model Context Protocol) 서버 구현 분리
- 툴 파라미터를 구조체로 그룹화
- HTTP 클라이언트 빌더 재사용

#### 프로세서 모듈 (`src/processor.rs`)
- 요청 처리 로직의 메인 오케스트레이터
- `ProcessingContext` 구조체로 공유 상태 관리
- `ResponseData` 구조체로 함수 파라미터 수 감소

### 2. 코드 품질 개선 (Code Quality Improvements)

#### 함수 파라미터 최적화
**이전:**
```rust
fn format_output(
    cli: &Cli,
    method: &str,
    url_str: &str,
    ip_addr: &str,
    status: StatusCode,
    size: u64,
    elapsed: Duration,
    title: &Option<String>,
    req_for_display: &Option<String>,
    body_text: &Option<String>,
) -> String
```

**이후:**
```rust
struct ResponseData<'a> {
    method: &'a str,
    url_str: &'a str,
    ip_addr: &'a str,
    status: StatusCode,
    size: u64,
    elapsed: Duration,
    title: &'a Option<String>,
    req_for_display: &'a Option<String>,
    body_text: &'a Option<String>,
}

fn format_response_output(cli: &Cli, data: &ResponseData) -> String
```

#### 상수 정의
- 매직 넘버 제거
- `constants.rs`에 모든 상수 집중화
- 문서화된 상수 사용

**예시:**
```rust
pub const DEFAULT_REDIRECT_LIMIT: usize = 10;
pub const HTTP_VERSION_2: &str = "HTTP/2.0";
pub const HTTP_VERSION_1_1: &str = "HTTP/1.1";
pub const MICROSECONDS_PER_SECOND: u64 = 1_000_000;
```

#### 에러 처리
- `unwrap()` 사용 최소화
- `unwrap_or_default()` 사용
- 명확한 에러 메시지

### 3. 테스트 추가 (Test Coverage)

총 17개의 유닛 테스트 추가:

```rust
// URL 정규화 테스트 (5개)
- test_normalize_url_with_scheme
- test_normalize_url_with_port_80
- test_normalize_url_with_port_443
- test_normalize_url_with_custom_port
- test_normalize_url_without_port

// 헤더 파싱 테스트 (2개)
- test_parse_headers
- test_parse_headers_invalid

// 요청 파싱 테스트 (3개)
- test_parse_request_line_get
- test_parse_request_line_post
- test_parse_request_line_empty

// 필터링 테스트 (3개)
- test_filter_by_status
- test_filter_by_string
- test_no_filter

// HTML 파싱 테스트 (2개)
- test_extract_title
- test_extract_title_no_title

// 출력 포맷팅 테스트 (2개)
- test_format_plain_output_no_template
- test_format_plain_output_with_template
```

### 4. 성능 개선 (Performance Improvements)

- 불필요한 클론 제거
- Arc/Mutex 사용 최적화
- 구조체로 데이터 그룹화하여 메모리 효율성 향상

### 5. 문서화 (Documentation)

- 모든 public 함수에 문서 주석 추가
- 모듈별 책임 명시
- 테스트 예시 포함
- `cargo doc` 지원

## 빌드 및 테스트 (Build and Test)

### 빌드
```bash
cargo build --release
```

### 테스트
```bash
cargo test
```

**결과**: ✅ 17 tests passed; 0 failed

### 린팅
```bash
cargo clippy --all-targets -- -D warnings
```

**결과**: ✅ No warnings

### 포맷팅
```bash
cargo fmt --check
```

**결과**: ✅ Formatted correctly

### 문서 생성
```bash
cargo doc --workspace --all-features --no-deps --document-private-items
```

**결과**: ✅ Documentation generated

## 이점 (Benefits)

### 개발자 경험 (Developer Experience)
1. **코드 탐색 용이**: 기능별로 파일이 분리되어 있어 원하는 코드를 빠르게 찾을 수 있음
2. **변경 영향도 감소**: 모듈화로 인해 한 부분의 변경이 다른 부분에 미치는 영향 최소화
3. **테스트 작성 용이**: 작은 함수와 명확한 책임으로 테스트 작성이 쉬워짐

### 유지보수성 (Maintainability)
1. **버그 수정 간편**: 문제가 발생한 모듈만 집중해서 수정 가능
2. **기능 추가 용이**: 새로운 출력 포맷이나 필터 타입 추가가 간단함
3. **코드 리뷰 개선**: 작은 단위로 분리되어 리뷰가 더 효과적

### 성능 (Performance)
1. **컴파일 시간**: 모듈화로 인한 증분 컴파일 개선
2. **실행 성능**: 구조체 사용으로 데이터 전달 최적화
3. **메모리 사용**: Arc/Mutex 사용 최적화

## 향후 개선 방향 (Future Improvements)

1. **더 많은 테스트**: 통합 테스트 및 E2E 테스트 추가
2. **벤치마크**: 성능 벤치마크 추가
3. **에러 타입**: 커스텀 에러 타입으로 에러 처리 개선
4. **비동기 최적화**: tokio 사용 최적화
5. **문서화**: 사용 예제 및 가이드 추가

## 결론 (Conclusion)

이번 리팩토링을 통해:
- ✅ 코드 품질이 크게 향상되었습니다
- ✅ 유지보수성이 개선되었습니다
- ✅ 테스트 커버리지가 추가되었습니다
- ✅ 코드 구조가 명확해졌습니다
- ✅ 개발자 경험이 향상되었습니다

모든 기능은 정상적으로 작동하며, 성능 저하 없이 코드 품질만 개선되었습니다.
