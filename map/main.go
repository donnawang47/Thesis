package main

import (
	"bytes"
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"net/http"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/lambda"
	"github.com/gorilla/mux"
)

type GenerateRouteRequest struct {
	Points [][]float64 `json:"points"`
}

type GenerateRouteResponse struct {
	Path [][]float64 `json:"path"`
}

func generateRouteLocalHandler(w http.ResponseWriter, r *http.Request) {
	log.Printf("1")

	// Parse the request body for points into a GenerateRouteRequest struct
	var req GenerateRouteRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		log.Println("Error decoding request:", err)
		return
	}

	// Prepare the JSON payload to send in the HTTP request
	payload, err := json.Marshal(req) // Marshal the body of the request
	if err != nil {
		http.Error(w, "Failed to marshal request body", http.StatusInternalServerError)
		log.Println("Error marshaling request body:", err)
		return
	}

	// Create the HTTP request to the external API
	apiURL := "http://localhost:9000/" // Replace with the external API URL
	httpReq, err := http.NewRequest("POST", apiURL, bytes.NewBuffer(payload))
	if err != nil {
		http.Error(w, "Failed to create HTTP request", http.StatusInternalServerError)
		log.Println("Error creating HTTP request:", err)
		return
	}

	// Set the appropriate headers for the request
	httpReq.Header.Set("Content-Type", "application/json")

	// Example of setting an Authorization header if needed
	// httpReq.Header.Set("Authorization", "Bearer YOUR_TOKEN")

	// Log the request details
	log.Printf("Sending %s request to %s", httpReq.Method, httpReq.URL.String())
	log.Println("Request Headers:")
	for name, values := range httpReq.Header {
		for _, value := range values {
			log.Printf("%s: %s", name, value)
		}
	}

	// Log the body before sending
	if httpReq.Body != nil {
		bodyBytes, err := io.ReadAll(httpReq.Body)
		if err != nil {
			log.Println("Error reading request body:", err)
		} else {
			log.Printf("Request Body: %s", string(bodyBytes))
			// Restore the body for further use
			httpReq.Body = io.NopCloser(bytes.NewReader(bodyBytes))
		}
	}

	// Create an HTTP client with a timeout (increase if necessary)
	client := &http.Client{
		Timeout: 30 * time.Second, // Set the timeout to 30 seconds
	}

	// Send the request to the external API
	resp, err := client.Do(httpReq)
	if err != nil {
		http.Error(w, "Failed to send HTTP request", http.StatusInternalServerError)
		log.Println("Error sending HTTP request:", err)
		return
	}
	defer resp.Body.Close()

	// Log the response status
	log.Printf("Received response with status: %s", resp.Status)

	// Check for successful response from the external API
	if resp.StatusCode != http.StatusOK {
		http.Error(w, "External API request failed", resp.StatusCode)
		log.Println("External API error:", resp.Status)
		return
	}

	// Read the response body from the external API
	var bodyResp GenerateRouteResponse
	if err := json.NewDecoder(resp.Body).Decode(&bodyResp); err != nil {
		http.Error(w, "Failed to decode API response", http.StatusInternalServerError)
		log.Println("Error decoding API response:", err)
		return
	}

	// Respond to the client
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)

	// Encoding the response to send back to the client
	if err := json.NewEncoder(w).Encode(bodyResp); err != nil {
		log.Println("Error encoding response:", err)
	}
}

func generateRouteHandler(w http.ResponseWriter, r *http.Request) {

	// Parse the request body for points
	var req GenerateRouteRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		log.Println("Error decoding request:", err)
		return
	}

	// Load AWS configuration
	cfg, err := config.LoadDefaultConfig(context.TODO())
	if err != nil {
		http.Error(w, "Failed to load AWS configuration", http.StatusInternalServerError)
		log.Println("Error loading AWS config:", err)
		return
	}

	// Create a Lambda client
	lambdaClient := lambda.NewFromConfig(cfg)

	// Prepare the Lambda input payload in the expected HTTP event format
	httpEvent := map[string]interface{}{
		"httpMethod": "POST",
		"headers": map[string]string{
			"Content-Type": "application/json",
		},
		"body":            string(mustMarshal(req)), // The JSON-encoded body of the request
		"isBase64Encoded": false,
	}

	payload, err := json.Marshal(httpEvent)
	if err != nil {
		http.Error(w, "Failed to marshal request payload", http.StatusInternalServerError)
		log.Println("Error marshaling payload:", err)
		return
	}

	log.Printf("Payload: %s", payload)

	// Invoke the Lambda function
	lambdaInput := &lambda.InvokeInput{
		FunctionName: aws.String("get-shortest-path"), // Replace with your Lambda function name
		Payload:      payload,
	}

	result, err := lambdaClient.Invoke(context.TODO(), lambdaInput)
	if err != nil {
		http.Error(w, "Failed to invoke Lambda function", http.StatusInternalServerError)
		log.Println("Error invoking Lambda function:", err)
		return
	}

	// Handle the Lambda response (directly parsing the body)
	var lambdaResp map[string]interface{}
	if err := json.Unmarshal(result.Payload, &lambdaResp); err != nil {
		http.Error(w, "Failed to parse Lambda response", http.StatusInternalServerError)
		log.Println("Error parsing Lambda response:", err)
		return
	}

	// Extract the body (which contains the actual data)
	bodyStr, ok := lambdaResp["body"].(string)
	if !ok {
		http.Error(w, "Invalid body in Lambda response", http.StatusInternalServerError)
		log.Println("Invalid body in Lambda response")
		return
	}

	// Parse the path from the body (which is a JSON string)
	var bodyResp GenerateRouteResponse
	if err := json.Unmarshal([]byte(bodyStr), &bodyResp); err != nil {
		http.Error(w, "Failed to parse body", http.StatusInternalServerError)
		log.Println("Error parsing body:", err)
		return
	}

	// Respond to the client
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)

	// Encoding the response to send back to the client
	if err := json.NewEncoder(w).Encode(bodyResp); err != nil {
		log.Println("Error encoding response:", err)
	}
}

// Helper function to marshal JSON and panic on error
func mustMarshal(v interface{}) []byte {
	data, err := json.Marshal(v)
	if err != nil {
		panic(err) // Handle this more gracefully in production code
	}
	return data
}

// Simple handler for basic requests (Ping or Debug)
func SimpleHandler(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	fmt.Fprintf(w, "Hello, world!")
	if flusher, ok := w.(http.Flusher); ok {
		flusher.Flush()
	}
}

// Main function
func main() {
	// Flag to enable/disable debug logging
	logFlag := flag.Bool("debug", true, "toggle debug logging on/off. Default is on.")
	flag.Parse()

	// Setting up log levels based on the flag
	if *logFlag {
		log.Println("Debug logging enabled")
	} else {
		log.Println("Info logging enabled")
	}

	// Set up your handlers and routes
	r := mux.NewRouter()

	// Serve static files from the 'static' folder
	r.PathPrefix("/static/").Handler(http.StripPrefix("/static/", http.FileServer(http.Dir("./static"))))

	// Serve the index.html file when accessing the root
	r.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.ServeFile(w, r, "./static/index.html")
	}).Methods("GET")

	// Path-related API handler

	r.HandleFunc("/route", generateRouteLocalHandler).Methods("POST", "GET")

	// Configure and start the server
	srv := &http.Server{
		Handler:      r,
		Addr:         ":8000",
		WriteTimeout: 1 * time.Minute,
		ReadTimeout:  1 * time.Minute,
	}

	log.Printf("Running server at: %v", srv.Addr)
	log.Fatal(srv.ListenAndServe())
}
